# PRD: Fase 7 - Escalonador M:N e Modelo Concorrente (CSP)

## 1. Objetivo da Fase
Implementar o motor de concorrência massiva da Kata-Lang, conhecido como *Green Threads*. Em vez de depender do Sistema Operacional para criar e escalar cada processo (o que é custoso), o `kata-runtime` abrigará um escalonador em tempo de execução (*Work-Stealing Scheduler*) que mapeará N *Actions* leves (CSP) sobre M *Threads* nativas da CPU. Adicionalmente, esta fase fornecerá a estrutura de Canais (Channels) tipada, permitindo a passagem de dados puramente assíncrona entre *Actions* isoladas, com segurança de memória alicerçada na Fase 6.

## 2. Escopo

**Dentro do Escopo:**
- **Kata Scheduler Crate (`kata-runtime/src/scheduler.rs`):**
  - Criação de um *ThreadPool* dinâmico escalado pelo número de núcleos físicos.
  - Implementação da Primitiva `fork!` via FFI, que aceitará um ponteiro de função gerado pelo Cranelift e o injetará na fila de tarefas do escalonador.
- **Topologia de Canais (`kata-runtime/src/channel.rs`):**
  - Construção de três estruturas de mensageria concorrente em memória baseadas em filas (MPSC e SPMC) para FFI:
    - **Rendezvous Channel (`channel!`):** Síncrono (tamanho 0). Envio e recepção bloqueiam-se mutuamente até a permuta ocorrer.
    - **Queue Channel (`queue!`):** Assíncrono com Buffer fixo. Envio só bloqueia se fila cheia.
    - **Broadcast Channel (`broadcast!`):** Assíncrono Publish/Subscribe (1 para N). *O canal em si não possui buffer central de retenção.* Ele atua como um hub. Quando um consumidor dá "subscribe", ele cria a sua *própria* fila (com um tamanho especificado). Se o hub enviar um dado e a fila de um consumidor específico estiver cheia, ocorre o *Drop-Oldest* **apenas para aquele consumidor**. O produtor nunca é bloqueado.
- **Integração de Memória Arc (Zero-Copy Inter-Process):**
  - Ao enviar um objeto gerado em uma `ThreadArena` (Fase 6) por um Canal, o canal chamará silenciosamente o `kata_rt_promote` para jogar o dado na *Global Heap* antes da permuta, e do outro lado da recepção, entregará o ponteiro global (via `kata_rt_release` pós consumo).

**Fora do Escopo:**
- **Diretiva @restart / Máquinas de Estado Cooperativas:** O modelo de interrupção não-preemptiva (onde o Cranelift gera yield points explicitos via Máquina de Estado em operações de I/O) exige uma arquitetura complexa de FFI "Coroutine" que depende fortemente de como o *Backend* (Fase 5) emitirá o código. Para esta Fase 7, as *Green Threads* serão implementadas rudimentarmente como *Closures/Threads Nativas do S.O* mapeadas 1:1, ou como tarefas assíncronas padrão, enquanto montamos o arcabouço FFI que será adaptado para M:N real na Fase 8/Final de integração com o Cranelift.
- A diretiva `@parallel` (que isola agressivamente a memória por processo do S.O) está fora do escopo. Focaremos no modo `spawn!` padrão (memória partilhada no mesmo processo).

## 3. Requisitos Técnicos

### 3.1. Arquitetura de Canais CSP (C/FFI Bound)
O código Assembly gerado pelo Cranelift não sabe instanciar canais. Ele vai evocar as seguintes APIs em C do Runtime:
- `kata_rt_chan_create(tipo, buffer_size) -> (*mut Tx, *mut Rx)`
- `kata_rt_chan_send(*mut Tx, *mut u8) -> bool` (Onde o `*mut u8` é o objeto Kata na Arena Local que precisa ser Promovido).
- `kata_rt_chan_recv(*mut Rx) -> *mut u8`

### 3.2. A Regra do Broadcast (Drop-Oldest)
O `broadcast!` é a única estrutura não contida e infinita. Se um produtor inserir dados na fila e não houver leitores suficientes rápidos, a fila não deve aumentar seu consumo de RAM. Ela removerá o ponteiro folha (*tail*), evocará o `kata_rt_release` sobre ele para abater seu *Reference Count*, e colocará o novo item na cabeça (*head*).

## 4. Estruturas de Dados Principais

As estruturas usarão nativamente canais bloqueantes seguros do Rust como base para simular a fila (Crossbeam ou Std MPSC), envelopados numa interface puramente opaca de C para o compilador.

```rust
// Tipos Opacos C para o Cranelift trafegar
pub struct KataSender {
    chan_type: u8,
    // [Ponteiros de Fila Nativa Ocultos]
}

pub struct KataReceiver {
    chan_type: u8,
    // [Ponteiros de Fila Nativa Ocultos]
}

#[repr(C)]
pub struct ChannelPair {
    tx: *mut KataSender,
    rx: *mut KataReceiver,
}

// FFI exportada para a Action iniciar Concorrência
#[no_mangle]
pub extern "C" fn kata_rt_fork(func_ptr: extern "C" fn(*mut u8), arg_ptr: *mut u8);
```

## 5. Critérios de Aceite

1. **Criação de Canais MPSC:** Invocar a FFI `kata_rt_chan_create(1, 10)` deve devolver um ponteiro `ChannelPair` alocado em RAM global sem falhar.
2. **Transferência de Propriedade (Passagem Assíncrona):** Um teste nativo no Runtime deve simular duas "threads": A Thread 1 aloca um dado falso na Arena e o envia (`kata_rt_chan_send`). O Runtime deve ser testado promovendo-o silenciosamente para a `Global Heap`. A Thread 2 deve receber o mesmo objeto chamando `kata_rt_chan_recv` e verificar que o ponteiro lido e modificado agora mora na Heap partilhada.
3. **O Descarte Silencioso (Broadcast):** Simular um envio de 5 mensagens numa fila de Broadcast de tamanho 3 para um Subscritor lento. O teste deve provar que as duas primeiras mensagens enviadas foram dropadas ativamente via ARC (`kata_rt_release` chamado) e o consumidor só leu as 3 mais recentes.