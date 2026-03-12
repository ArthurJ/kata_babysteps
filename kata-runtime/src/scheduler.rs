use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

// A engine estática global do Tokio para agendar todas as Green Threads da Kata
static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

/// Retorna a instância compartilhada do Tokio que hospeda as Green Threads.
/// Ela é inicializada de forma lenta (lazy) na primeira vez que uma Action precisa de concorrência.
pub fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(num_cpus::get()) // M:N (N workers nativos do SO)
            .build()
            .expect("Kata Runtime: Falha ao inicializar o motor Tokio")
    })
}

// ==============================================================
// PRMITIVA DE CONCORRÊNCIA (FFI do fork!)
// ==============================================================

/// Dispara uma nova Green Thread.
/// Recebe um ponteiro de função gerado pelo Cranelift (que atua como a rotina da Action).
/// `arg_ptr` é um ponteiro opcional genérico de dados (uma tupla promovida via ARC na Heap) 
/// a ser desempacotado na Action filha.
#[no_mangle]
pub extern "C" fn kata_rt_fork(
    func_ptr: extern "C" fn(*mut u8), 
    arg_ptr: *mut u8
) {
    let rt = get_runtime();
    
    // O Rust considera ponteiros C inseguros para mover entre threads (trait Send).
    // Para resolver isso, convertemos temporariamente os ponteiros para `usize` (que é Send).
    // O Runtime Kata garante o isolamento da memória e a segurança dessa operação.
    let func_addr = func_ptr as usize;
    let arg_addr = arg_ptr as usize;

    rt.spawn(async move {
        // Transmuta de volta para ponteiros C dentro da nova Green Thread
        let func: extern "C" fn(*mut u8) = unsafe { std::mem::transmute(func_addr) };
        let arg = arg_addr as *mut u8;
        
        // Executa o assembly C puro na Green Thread com o argumento
        func(arg);
    });
}
