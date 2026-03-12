# PRD: Fase 6 - O Runtime Base e Topologia de Memória

## 1. Objetivo da Fase
Projetar e construir o motor de gestão de memória subjacente (Kata Runtime) que será embutido nos executáveis da linguagem. Esta fase implementa o modelo de alocação especificado no *Specs Draft*, que recusa a varredura convencional (*Tracing Garbage Collector*) em prol de um sistema ultrarrápido baseado em **Arenas Locais de Custo Zero** para dados puramente imutáveis, acoplado a uma **Global Heap com ARC (Atomic Reference Counting)** utilizada estritamente quando dados "escapam" dos processos através de canais.

## 2. Escopo

**Dentro do Escopo:**
- **Kata Runtime Crate:** Criação do módulo `kata-runtime` em Rust que exporá uma API em C (FFI) nativa para ser chamada pelo código Assembly gerado pelo Cranelift.
- **Topologia de Arena Local (Bump Allocator):**
  - Implementação de um alocador sequencial contíguo alocado no escopo de cada *Action* / *Processo*.
  - Descarte em $O(1)$ (*Drop* instantâneo de todo o bloco de memória quando a *Action* termina).
- **Cabeçalho de Objeto (*Object Header*):**
  - Todos os objetos gerados na *Arena* deverão conter um metadado invisível (Cabeçalho de Encaminhamento) preparado para a eventual "Promoção".
- **Global Heap & Promoção ARC:**
  - Lógica para capturar uma variável residente na *Arena*, copiá-la fisicamente para a Memória Global Partilhada (*Global Heap*) e aplicar *Atomic Reference Counting*.
  - Gravação de um *Forwarding Pointer* no cabeçalho do objeto original na *Arena*, informando que aquele dado foi promovido.
- **Limpeza Atômica:** Desalocação do bloco da *Heap Global* quando o ARC atinge zero.

**Fora do Escopo:**
- O Escalonador de *Green Threads* / M:N e o Modelo Concorrente de Canais (CSP) serão implementados inteiramente na **Fase 7**. O motor de memória providenciado aqui na Fase 6 apenas prepara a base estrutural para a partilha paralela segura que será consumida depois.
- Chamadas a Sistema (I/O Sockets/Arquivos) também não serão tratadas no Runtime até a Fase 8.

## 3. Requisitos Técnicos

### 3.1. Alocador de Arena O(1)
A natureza puramente imutável do domínio Funcional da Kata-Lang garante que as referências locais não gerem corrupção.
- O Runtime inicializará uma *Arena* (ex: array de 4MB na RAM) no momento em que uma nova *Action* arrancar.
- Sempre que uma coleção complexa for instanciada (ex: `List` ou `Tuple`), um ponteiro é devolvido simplesmente avançando o índice de limite da Arena (Bump Allocation).
- Como não há *Aliasing* Mutável de memória livre, e as Funções puras não geram ciclos de referência complexos sem retorno, as *Arenas* jamais requerem desfragmentação. Ao fim da execução da *Action*, o `cursor` da Arena apenas volta ao 0, libertando todos os nós alocados de uma só vez num custo de CPU virtualmente nulo.

### 3.2. Estrutura do Cabeçalho e Forwarding
Todo bloco de dado alocado em uma *Arena* deve iniciar com uma tag de 64-bits (*usize*):
- `0` = Objeto puramente local.
- `[Ponteiro_Valido]` = O Objeto escapou para a *Heap Global*. A partir deste momento, leituras subsequentes feitas por outras *Actions* devem seguir este ponteiro (Forwarding).
Quando a instrução nativa do Cranelift for enviar algo por um `channel!` (Fase 7), ela evocará `kata_rt_promote(ptr)`.

## 4. Estruturas de Dados Principais

```rust
// Interface FFI que será exposta e embutida no Linker pelo Cranelift

#[repr(C)]
pub struct KataHeader {
    // 0 = Nunca Promovido. Caso contrário, contém o ponteiro para a Global Heap.
    forwarding_ptr: usize, 
    // Tipo do objeto para que o Runtime saiba quanto copiar durante a promoção.
    size: u32,
    type_tag: u32,
}

pub struct ThreadArena {
    buffer: *mut u8,
    capacity: usize,
    cursor: usize,
}

impl ThreadArena {
    pub fn alloc(&mut self, size: usize) -> *mut u8;
    pub fn reset(&mut self);
}

// O Recipiente ARC gerido pela Global Heap
pub struct GlobalArcNode {
    ref_count: std::sync::atomic::AtomicUsize,
    payload_size: usize,
    // [dados em bytes seguem sequencialmente na memoria]
}
```

## 5. Critérios de Aceite

1. **Inicialização Limpa:** A crate `kata-runtime` deve compilar independentemente como uma biblioteca estática (`libkata_runtime.a` ou `.rlib`).
2. **Alocação Local Veloz:** Um teste interno garantirá que invocar `kata_rt_alloc` mil vezes apenas incrementará o cursor contíguo da `ThreadArena` sem recorrer ao alocador nativo do S.O.
3. **Promoção Segura:** A invocação simulada de `kata_rt_promote(ptr)` sobre um objeto na *Arena* deve copiá-lo para a *Global Heap*, setar o `ref_count` para 1, e registrar o *Forwarding Pointer* no cabeçalho da Arena com precisão.
4. **Descarte ARC Seguro:** Invocar `kata_rt_release(global_ptr)` deve decrementar o `ref_count` atômico. Se bater 0, a memória da *Global Heap* deve ser desalocada e devolvida ao S.O. sem falhas de segmentação.