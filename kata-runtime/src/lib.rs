pub mod channel;
pub mod memory;
pub mod scheduler;
pub mod io;

#[cfg(test)]
mod tests {
    use super::memory::*;
    use super::channel::*;
    use std::sync::atomic::Ordering;
    use std::ptr;

    #[test]
    fn test_arena_allocation() {
        let mut arena = ThreadArena::new(1024 * 1024); // 1 MB
        
        let ptr1 = arena.alloc(16, 1);
        let ptr2 = arena.alloc(32, 2);

        unsafe {
            let h1 = ptr1 as *mut KataHeader;
            assert_eq!((*h1).size, 16);
            assert_eq!((*h1).type_tag, 1);
            assert_eq!((*h1).forwarding_ptr, 0);

            let h2 = ptr2 as *mut KataHeader;
            assert_eq!((*h2).size, 32);
            assert_eq!((*h2).type_tag, 2);
        }
    }

    #[test]
    fn test_arc_promotion() {
        let mut arena = ThreadArena::new(1024);
        let ptr = arena.alloc(8, 99);
        
        unsafe {
            let payload = ptr.add(KataHeader::SIZE);
            ptr::write(payload as *mut u64, 424242);

            let global_ptr = kata_rt_promote(ptr);
            assert_ne!(ptr, global_ptr);

            let header = ptr as *mut KataHeader;
            assert_eq!((*header).forwarding_ptr, global_ptr as usize);

            let arc_node = global_ptr.sub(GlobalArcNode::SIZE) as *mut GlobalArcNode;
            assert_eq!((*arc_node).ref_count.load(Ordering::SeqCst), 1);

            let global_payload = global_ptr.add(KataHeader::SIZE);
            assert_eq!(ptr::read(global_payload as *mut u64), 424242);

            let global_ptr_2 = kata_rt_promote(ptr);
            assert_eq!(global_ptr, global_ptr_2);
            assert_eq!((*arc_node).ref_count.load(Ordering::SeqCst), 2);

            kata_rt_release(global_ptr);
            assert_eq!((*arc_node).ref_count.load(Ordering::SeqCst), 1);
            
            kata_rt_release(global_ptr);
        }
    }

    #[test]
    fn test_channel_mpsc_zero_copy() {
        // Simula o runtime Kata instanciando um canal
        let pair = kata_rt_chan_create_queue(10);
        let mut arena = ThreadArena::new(1024);
        
        unsafe {
            // Action 1: Cria um objeto local e envia
            let local_obj = arena.alloc(8, 1);
            let p1 = local_obj.add(KataHeader::SIZE);
            ptr::write(p1 as *mut u64, 777);

            // O envio deve disparar a promoção silenciosamente!
            let success = kata_rt_chan_send(pair.tx, local_obj);
            assert!(success);

            // Verifica se a Arena acusou a promoção
            let header = local_obj as *mut KataHeader;
            assert_ne!((*header).forwarding_ptr, 0);
            let global_addr = (*header).forwarding_ptr as *mut u8;

            // Action 2: Recebe do outro lado
            let recv_obj = kata_rt_chan_recv(pair.rx);
            assert_ne!(recv_obj, ptr::null_mut());
            
            // O ponteiro recebido DEVE ser exatamente o ponteiro global (Zero-Copy Inter-Process)
            assert_eq!(recv_obj, global_addr);

            // Limpeza
            kata_rt_release(recv_obj);
            let _ = Box::from_raw(pair.tx);
            let _ = Box::from_raw(pair.rx);
        }
    }

    #[test]
    fn test_channel_broadcast_drop_oldest() {
        let pair = kata_rt_chan_create_broadcast();
        
        unsafe {
            // Inscreve um consumidor com uma fila minúscula (capacidade teórica menor que os envios)
            // No Tokio, a criação do broadcast define o buffer global para todos, 
            // mas o Lagged ocorre na leitura. O buffer do nosso Hub é 1024,
            // mas para este teste forçaremos um envio massivo e rápido sem ler,
            // e verificaremos se o Tokio emite Lagged ao tentar recuperar o atraso.
            // Para simplificar o teste FFI, vamos criar um canal broadcast de tamanho 2 direto:
            let (tx, rx) = tokio::sync::broadcast::channel(2);
            let tx_ptr = Box::into_raw(Box::new(AnySender::Broadcast(KataSenderBroadcast { tx })));
            let rx_ptr = Box::into_raw(Box::new(AnyReceiver::Broadcast(KataReceiverBroadcast { rx })));
            
            let mut arena = ThreadArena::new(1024);

            // Produtor Rápido: Envia 5 mensagens
            for i in 1..=5 {
                let obj = arena.alloc(8, i); // Usamos o tipo_tag como contador para validar
                let p = obj.add(KataHeader::SIZE);
                ptr::write(p as *mut u64, i as u64);
                kata_rt_chan_send(tx_ptr, obj);
            }

            // Consumidor Lento Acorda:
            // A fila tem tamanho 2. Enviamos 5. Os 3 primeiros devem ter sido ejetados.
            // A nossa função kata_rt_chan_recv está configurada para ENGOLIR o erro Lagged
            // e retornar a próxima válida (que será a 4).
            
            let recv_obj_1 = kata_rt_chan_recv(rx_ptr);
            assert_ne!(recv_obj_1, ptr::null_mut());
            let h1 = recv_obj_1 as *mut KataHeader;
            assert_eq!((*h1).type_tag, 4); // Recebeu a 4ª mensagem! A 1, 2 e 3 sumiram.

            let recv_obj_2 = kata_rt_chan_recv(rx_ptr);
            assert_ne!(recv_obj_2, ptr::null_mut());
            let h2 = recv_obj_2 as *mut KataHeader;
            assert_eq!((*h2).type_tag, 5); // Recebeu a 5ª.

            // Limpeza manual para não vazar o teste
            let _ = Box::from_raw(tx_ptr);
            let _ = Box::from_raw(rx_ptr);
        }
    }
}
pub mod collections;
