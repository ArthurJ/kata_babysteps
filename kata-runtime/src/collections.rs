use std::ffi::CString;
use std::os::raw::c_char;
use crate::memory::KataHeader;

// Estrutura que reflete a Tupla que o compilador AOT monta no MakeTuple
#[repr(C)]
pub struct KataRange {
    pub start: i64,
    pub step: i64, // o `..`
    pub end: i64,
}

#[no_mangle]
pub extern "C" fn kata_rt_mock_list(tuple_ptr: *mut u8) -> *mut u8 {
    // Nós recebemos um ponteiro bruto da ThreadArena.
    // Pela especificação atual do TypeChecker, ranges [1..15]
    // são transformados em uma Tupla de 3 i64.
    
    unsafe {
        let range = &*(tuple_ptr as *const KataRange);
        
        let start = range.start;
        let end = range.end;
        
        let count = (end - start + 1).max(0) as usize;
        
        // Aloca uma nova Lista na Arena
        let type_tag = 2; // Tag arbitrária pra List::Int
        let list_ptr = crate::memory::kata_rt_alloc((count * 8) as u32, type_tag);
        
        let mut current_ptr = list_ptr as *mut i64;
        for i in start..=end {
            *current_ptr = i;
            current_ptr = current_ptr.add(1);
        }
        
        list_ptr
    }
}

// Assinatura do Callback que o Cranelift gera para a lambda do map
type MapCallback = extern "C" fn(i64) -> *mut c_char;

#[no_mangle]
pub extern "C" fn kata_rt_mock_map(func_ptr: MapCallback, list_ptr: *mut u8) -> *mut u8 {
    unsafe {
        // Encontra o tamanho original do bloco de memória olhando o cabeçalho retroativo
        let header_ptr = list_ptr.sub(std::mem::size_of::<KataHeader>()) as *const KataHeader;
        let count = (*header_ptr).size as usize / 8;
        
        // Debug: printf the count to see why it prints only 3 elements
        // println!("Map count: {}, List ptr size: {}", count, (*header_ptr).size);
        
        // Aloca uma nova lista para os ponteiros de Text (c_char)
        let new_list_ptr = crate::memory::kata_rt_alloc((count * 8) as u32, 3 /* Tag List::Text */);
        
        let src_array = list_ptr as *const i64;
        let dst_array = new_list_ptr as *mut *mut c_char;
        
        for i in 0..count {
            let val = *src_array.add(i);
            // Invoca o código de máquina do `fizzbuzz` para cada inteiro!
            let text_ptr = func_ptr(val); 
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