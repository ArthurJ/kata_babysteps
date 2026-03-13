use std::ffi::CString;
use std::os::raw::c_char;
use crate::memory::KataHeader;
use crate::closure::KataClosure;

// Estrutura que reflete a Tupla que o compilador AOT monta no MakeTuple
#[repr(C)]
pub struct KataRange {
    pub start: i64,
    pub step: i64, // o `..`
    pub end: i64,
}

/// Cria uma lista a partir de um range [start..end] com step.
/// Aloca na arena global e retorna ponteiro para o primeiro elemento.
#[no_mangle]
pub extern "C" fn kata_rt_list_from_range(
    start: i64,
    step: i64,
    end: i64,
    elem_size: u32,
    type_tag: u32,
) -> *mut u8 {
    // Calcula quantidade de elementos
    let count = if step == 0 {
        0
    } else if step > 0 {
        ((end - start) / step + 1).max(0) as usize
    } else {
        ((start - end) / (-step) + 1).max(0) as usize
    };

    if count == 0 {
        // Retorna ponteiro nulo para lista vazia
        return std::ptr::null_mut();
    }

    unsafe {
        // Aloca a lista na arena global
        let list_ptr = crate::memory::kata_rt_alloc(count as u32 * elem_size, type_tag);

        // Preenche com valores sequenciais
        let elem_ptr = list_ptr as *mut i64;
        for i in 0..count {
            *elem_ptr.add(i) = start + (i as i64) * step;
        }

        list_ptr
    }
}

/// Aplica uma closure a cada elemento de uma lista, retornando uma nova lista.
/// A closure é passada por KataClosure (code_ptr + env_ptr).
/// Retorna um ponteiro para nova lista alocada na arena global.
#[no_mangle]
pub extern "C" fn kata_rt_map(
    closure: *mut KataClosure,
    list_ptr: *mut u8,
) -> *mut u8 {
    unsafe {
        if list_ptr.is_null() || closure.is_null() {
            return std::ptr::null_mut();
        }

        // Extrai o code_ptr e env_ptr da closure
        let code_ptr = (*closure).code_ptr;
        let env_ptr = (*closure).env_ptr;

        // O code_ptr é uma função que recebe (env, i64) -> *mut c_char
        let func: extern "C" fn(*mut u8, i64) -> *mut c_char =
            std::mem::transmute::<*mut u8, extern "C" fn(*mut u8, i64) -> *mut c_char>(code_ptr);

        // Encontra o tamanho original do bloco de memória olhando o cabeçalho retroativo
        let header_ptr = list_ptr.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;
        let count = (*header_ptr).size as usize / 8;
        let src_type_tag = (*header_ptr).type_tag;

        // Aloca uma nova lista para os resultados
        // Preserva o type_tag da lista original ou usa 3 para Text
        let result_type_tag = 3; // Tag List::Text
        let new_list_ptr = crate::memory::kata_rt_alloc((count * 8) as u32, result_type_tag);

        let src_array = list_ptr as *const i64;
        let dst_array = new_list_ptr as *mut *mut c_char;

        for i in 0..count {
            let val = *src_array.add(i);
            // Invoca a closure: func(env_ptr, val)
            let text_ptr = func(env_ptr, val);
            *dst_array.add(i) = text_ptr;
        }

        new_list_ptr
    }
}

#[no_mangle]
pub extern "C" fn kata_rt_list_to_str(list_ptr: *mut u8) -> *mut c_char {
    if list_ptr.is_null() {
        return CString::new("[]").unwrap().into_raw();
    }
    
    unsafe {
        let header_ptr = list_ptr.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;
        let count = (*header_ptr).size as usize / 8;
        
        let type_tag = (*header_ptr).type_tag;
        let mut result = String::from("[");
        
        if type_tag == 3 { // List::Text
            let text_ptrs = list_ptr as *const *const c_char;
            for i in 0..count {
                let s_ptr = *text_ptrs.add(i);
                if !s_ptr.is_null() {
                    let c_str = std::ffi::CStr::from_ptr(s_ptr);
                    if let Ok(s) = c_str.to_str() {
                        result.push_str(s);
                    }
                } else {
                    result.push_str("null");
                }
                if i < count - 1 { result.push_str(", "); }
            }
        } else { // Ints
            let int_ptrs = list_ptr as *const i64;
            for i in 0..count {
                let val = *int_ptrs.add(i);
                result.push_str(&val.to_string());
                if i < count - 1 { result.push_str(", "); }
            }
        }
        
        result.push_str("]");
        CString::new(result).unwrap().into_raw()
    }
}

/// Filtra elementos de uma lista baseado em um predicado (closure que retorna Bool).
/// Retorna um ponteiro para nova lista alocada na arena global.
#[no_mangle]
pub extern "C" fn kata_rt_filter(
    closure: *mut KataClosure,
    list_ptr: *mut u8,
) -> *mut u8 {
    unsafe {
        if list_ptr.is_null() || closure.is_null() {
            return std::ptr::null_mut();
        }

        // Extrai o code_ptr e env_ptr da closure
        let code_ptr = (*closure).code_ptr;
        let env_ptr = (*closure).env_ptr;

        // O code_ptr é uma função que recebe (env, i64) -> bool
        let func: extern "C" fn(*mut u8, i64) -> i64 =
            std::mem::transmute::<*mut u8, extern "C" fn(*mut u8, i64) -> i64>(code_ptr);

        // Encontra o tamanho original da lista
        let header_ptr = list_ptr.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;
        let count = (*header_ptr).size as usize / 8;
        let type_tag = (*header_ptr).type_tag;

        // Primeiro passo: conta quantos elementos passam no filtro
        let src_array = list_ptr as *const i64;
        let mut matching_count = 0;
        for i in 0..count {
            let val = *src_array.add(i);
            let result = func(env_ptr, val);
            if result != 0 {
                matching_count += 1;
            }
        }

        if matching_count == 0 {
            return std::ptr::null_mut();
        }

        // Aloca nova lista com os elementos filtrados
        let new_list_ptr = crate::memory::kata_rt_alloc((matching_count * 8) as u32, type_tag);
        let dst_array = new_list_ptr as *mut i64;

        // Segundo passo: copia os elementos que passaram no filtro
        let mut dst_idx = 0;
        for i in 0..count {
            let val = *src_array.add(i);
            let result = func(env_ptr, val);
            if result != 0 {
                *dst_array.add(dst_idx) = val;
                dst_idx += 1;
            }
        }

        new_list_ptr
    }
}

/// Aplica uma operação acumulativa (fold/reduce) sobre uma lista.
/// Recebe: closure (B A -> B), valor inicial B, lista A
/// Retorna: valor acumulado B
#[no_mangle]
pub extern "C" fn kata_rt_fold(
    closure: *mut KataClosure,
    initial: i64,
    list_ptr: *mut u8,
) -> i64 {
    unsafe {
        if list_ptr.is_null() || closure.is_null() {
            return initial;
        }

        // Extrai o code_ptr e env_ptr da closure
        let code_ptr = (*closure).code_ptr;
        let env_ptr = (*closure).env_ptr;

        // O code_ptr é uma função que recebe (env, acc, val) -> acc'
        let func: extern "C" fn(*mut u8, i64, i64) -> i64 =
            std::mem::transmute::<*mut u8, extern "C" fn(*mut u8, i64, i64) -> i64>(code_ptr);

        // Encontra o tamanho da lista
        let header_ptr = list_ptr.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;
        let count = (*header_ptr).size as usize / 8;

        let src_array = list_ptr as *const i64;
        let mut acc = initial;

        for i in 0..count {
            let val = *src_array.add(i);
            acc = func(env_ptr, acc, val);
        }

        acc
    }
}

/// Combina duas listas em uma lista de tuplas (pares).
/// Se as listas tiverem tamanhos diferentes, usa o menor tamanho.
#[no_mangle]
pub extern "C" fn kata_rt_zip(
    list_a: *mut u8,
    list_b: *mut u8,
) -> *mut u8 {
    unsafe {
        if list_a.is_null() || list_b.is_null() {
            return std::ptr::null_mut();
        }

        // Encontra o tamanho das listas
        let header_a = list_a.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;
        let header_b = list_b.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;

        let count_a = (*header_a).size as usize / 8;
        let count_b = (*header_b).size as usize / 8;
        let min_count = count_a.min(count_b);

        if min_count == 0 {
            return std::ptr::null_mut();
        }

        // Aloca nova lista para os pares (cada par ocupa 16 bytes - 2 x i64)
        let type_tag = 5; // Tag para List::Tuple
        let new_list_ptr = crate::memory::kata_rt_alloc((min_count * 16) as u32, type_tag);

        let src_a = list_a as *const i64;
        let src_b = list_b as *const i64;
        let dst = new_list_ptr as *mut i64;

        for i in 0..min_count {
            let val_a = *src_a.add(i);
            let val_b = *src_b.add(i);
            // Armazena o par sequencialmente (val_a, val_b)
            *dst.add(i * 2) = val_a;
            *dst.add(i * 2 + 1) = val_b;
        }

        new_list_ptr
    }
}