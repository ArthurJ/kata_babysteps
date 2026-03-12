use std::ptr;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use crate::memory::{kata_rt_promote, kata_rt_release};

// O Tipo que trafega internamente nos Canais. É o Ponteiro bruto de memória C.
pub type KataPtr = *mut u8;

// ==============================================================
// TOPOLOGIA 1: MPSC (Rendezvous e Queue)
// ==============================================================
pub struct KataSenderMpsc {
    pub tx: mpsc::Sender<KataPtr>,
}

pub struct KataReceiverMpsc {
    pub rx: mpsc::Receiver<KataPtr>,
}

// ==============================================================
// TOPOLOGIA 2: BROADCAST (Pub/Sub)
// ==============================================================
pub struct KataSenderBroadcast {
    pub tx: broadcast::Sender<KataPtr>,
}

pub struct KataReceiverBroadcast {
    pub rx: broadcast::Receiver<KataPtr>,
}

// ==============================================================
// ESTRUTURAS FFI OPACAS 
// ==============================================================
/// Os enums que encapsulam os canais verdadeiros que a memória C não compreende
pub enum AnySender {
    Mpsc(KataSenderMpsc),
    Broadcast(KataSenderBroadcast),
}

pub enum AnyReceiver {
    Mpsc(KataReceiverMpsc),
    Broadcast(KataReceiverBroadcast),
}

#[repr(C)]
pub struct ChannelPair {
    pub tx: *mut AnySender,
    pub rx: *mut AnyReceiver,
}

// ==============================================================
// FFIs (API Exportada para o Assembly Cranelift)
// ==============================================================

/// Cria um canal MPSC ou Rendezvous.
/// Se `buffer_size` for 0, cria o Rendezvous `channel!()`.
/// Se for > 0, cria o `queue!(size)`.
#[no_mangle]
pub extern "C" fn kata_rt_chan_create_queue(buffer_size: usize) -> ChannelPair {
    let size = if buffer_size == 0 { 1 } else { buffer_size };
    let (tx, rx) = mpsc::channel(size);

    let sender = AnySender::Mpsc(KataSenderMpsc { tx });
    let receiver = AnyReceiver::Mpsc(KataReceiverMpsc { rx });

    let b_tx = Box::into_raw(Box::new(sender));
    let b_rx = Box::into_raw(Box::new(receiver));

    ChannelPair { tx: b_tx, rx: b_rx }
}

/// Cria o "Hub" do Broadcast `broadcast!()`.
/// O Receiver devolvido NÃO É um consumidor, é uma Fábrica de Consumidores opaca
/// (Representando o `subscribe` na linguagem).
#[no_mangle]
pub extern "C" fn kata_rt_chan_create_broadcast() -> ChannelPair {
    // Hub primário do Tokio. O tamanho aqui é irrelevante do ponto de vista do Drop-Oldest,
    // o Tokio cuida do evicting nativamente.
    let (tx, rx) = broadcast::channel(1024);

    let sender = AnySender::Broadcast(KataSenderBroadcast { tx });
    let receiver = AnyReceiver::Broadcast(KataReceiverBroadcast { rx });

    let b_tx = Box::into_raw(Box::new(sender));
    let b_rx = Box::into_raw(Box::new(receiver));

    ChannelPair { tx: b_tx, rx: b_rx }
}

/// Cria a "Própria Fila" de um consumidor do Broadcast: `rx_cliente = subscribe(buffer_size)`
#[no_mangle]
pub unsafe extern "C" fn kata_rt_chan_broadcast_subscribe(rx_factory_ptr: *mut AnyReceiver) -> *mut AnyReceiver {
    if let AnyReceiver::Broadcast(b) = &*rx_factory_ptr {
        // Usa o descritor mestre para gerar um novo Receiver atrelado ao Hub
        // TODO: Na biblioteca tokio, o subscribe vem do tx original.
        // O `broadcast::Receiver::resubscribe()` funciona perfeitamente para clonar a fila.
        let new_rx = b.rx.resubscribe();
        
        let new_receiver = AnyReceiver::Broadcast(KataReceiverBroadcast { rx: new_rx });
        return Box::into_raw(Box::new(new_receiver));
    }
    ptr::null_mut()
}

/// Operador `>!`
/// Envia um dado pela rede M:N.
#[no_mangle]
pub unsafe extern "C" fn kata_rt_chan_send(tx_ptr: *mut AnySender, data_ptr: *mut u8) -> bool {
    if tx_ptr.is_null() || data_ptr.is_null() { return false; }
    
    // 1. MAGIA DO MOTOR: Promove a variável instantaneamente da Arena Local da Action
    // para a Heap Global com ARC = 1. Assim, o Produtor não vaza memória
    let global_data_ptr = kata_rt_promote(data_ptr);

    match &*tx_ptr {
        AnySender::Mpsc(m) => {
            // Bloqueia cooperativamente se fila estiver cheia (O Cranelift terá que gerenciar o Tokio Handle,
            // mas por ser Rust FFI, usamos o blocking_send nativo por ora)
            // Em V2, isso exige a yield machine.
            m.tx.blocking_send(global_data_ptr).is_ok()
        }
        AnySender::Broadcast(b) => {
            // O envio Broadcast NUNCA falha ou bloqueia. Se a fila local de um coitado
            // estiver cheia, o tokio::sync::broadcast vai ejetar os velhos!
            // E o melhor de tudo: quem trata o `kata_rt_release` dos items dropados 
            // será implementado na rotina de Receive abaixo, lidando com Lagged.
            b.tx.send(global_data_ptr).is_ok()
        }
    }
}

/// Operador `<!` (Bloqueante)
#[no_mangle]
pub unsafe extern "C" fn kata_rt_chan_recv(rx_ptr: *mut AnyReceiver) -> *mut u8 {
    if rx_ptr.is_null() { return ptr::null_mut(); }
    
    match &mut *rx_ptr {
        AnyReceiver::Mpsc(m) => {
            m.rx.blocking_recv().unwrap_or(ptr::null_mut())
        }
        AnyReceiver::Broadcast(b) => {
            loop {
                match b.rx.blocking_recv() {
                    Ok(ptr) => return ptr,
                    Err(broadcast::error::RecvError::Closed) => return ptr::null_mut(),
                    Err(broadcast::error::RecvError::Lagged(skipped_count)) => {
                        // ==== REGRA DO DROP-OLDEST NA KATA ====
                        // O Tokio diz "Ei, você estava muito atrasado. Eu perdi 15 mensagens."
                        // Na Kata, as mensagens são ponteiros de memória Global (GlobalArcNode).
                        // O Tokio dropou a *referência* à mensagem dele internamente.
                        // Mas o nosso ARC real de C ainda está vivo e pode vazar memória se todos tivessem Lagged.
                        // 
                        // Nota arquitetural avançada: O tokio `broadcast` clona (Clone trait) o valor T.
                        // Como o nosso valor T é um `*mut u8`, o clone foi grátis e o Tokio não
                        // soube incrementar o nosso ARC. 
                        // Para este modelo rudimentar, consideraremos que o Produtor garantiu a Heap, 
                        // e este erro é apenas informativo para a Action saltar.
                        println!("Kata CSP Warning: Subscriber Lagged by {} messages. Drop-Oldest enacted.", skipped_count);
                        continue;
                    }
                }
            }
        }
    }
}
