use std::cell::RefCell;

// ==========================================
// 1. Estruturas Base de Memória (Specs da Kata)
// ==========================================

#[repr(C)]
pub struct KataHeader {
    // 0 = Nunca Promovido. Caso contrário, ponteiro para a Global Heap.
    pub forwarding_ptr: usize, 
    pub size: u32,
    pub type_tag: u32,
}

pub struct ThreadArena {
    buffer: Vec<u8>,
    capacity: usize,
    cursor: usize,
}

impl ThreadArena {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        // Preenche com zeros para evitar lixo
        buffer.resize(capacity, 0);
        Self {
            buffer,
            capacity,
            cursor: 0,
        }
    }

    /// Expande a capacidade da arena para acomodar alocações adicionais
    fn expand(&mut self, needed: usize) {
        // Calcula novo tamanho: pelo menos o dobro do atual, ou o necessário
        let new_capacity = (self.capacity * 2).max(self.cursor + needed).max(4 * 1024 * 1024);

        eprintln!("DEBUG: Expandindo ThreadArena de {} para {} bytes", self.capacity, new_capacity);

        // Redimensiona o buffer
        self.buffer.resize(new_capacity, 0);
        self.capacity = new_capacity;
    }

    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        // Alinhamento em 8 bytes (padrão 64-bits)
        let align_offset = (8 - (self.cursor % 8)) % 8;
        let start = self.cursor + align_offset;
        let end = start + size;

        // Se não há espaço suficiente, expande a arena
        if end > self.capacity {
            self.expand(end - self.capacity);
        }

        self.cursor = end;
        unsafe { self.buffer.as_mut_ptr().add(start) }
    }

    pub fn reset(&mut self) {
        self.cursor = 0; // Drop imediato de TUDO (O(1))
    }
}

// ==========================================
// 2. Thread-Local State (A "Stack" do Escalonador)
// ==========================================
// Cada Action será emparelhada com uma Arena de 4MB.
thread_local! {
    pub static CURRENT_ARENA: RefCell<ThreadArena> = RefCell::new(ThreadArena::new(4 * 1024 * 1024));
}

// ==========================================
// 3. FFI's de Exportação para o Cranelift
// ==========================================

#[no_mangle]
pub extern "C" fn kata_rt_alloc(size: u32, type_tag: u32) -> *mut u8 {
    let total_size = std::mem::size_of::<KataHeader>() + (size as usize);
    
    CURRENT_ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let ptr = arena.alloc(total_size);
        
        // Inicializa o Header
        let header = ptr as *mut KataHeader;
        unsafe {
            (*header).forwarding_ptr = 0;
            (*header).size = size;
            (*header).type_tag = type_tag;
            
            // Retorna o ponteiro Pós-Header (Onde os dados reais começam)
            ptr.add(std::mem::size_of::<KataHeader>())
        }
    })
}

#[no_mangle]
pub extern "C" fn kata_rt_reset_arena() {
    CURRENT_ARENA.with(|arena| {
        arena.borrow_mut().reset();
    });
}



// Promotores Globais vazios por enquanto
pub fn kata_rt_promote(ptr: *mut u8) -> *mut u8 { ptr }
pub fn kata_rt_release(ptr: *mut u8) {}
