use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn kata_rt_print_str(str_ptr: *const c_char) {
    if str_ptr.is_null() {
        println!("(null)");
        return;
    }
    
    // Converte a C-String pra Rust String com segurança de ponteiro
    let c_str = unsafe { CStr::from_ptr(str_ptr) };
    if let Ok(s) = c_str.to_str() {
        println!("{}", s);
    } else {
        println!("(invalid utf8)");
    }
}

/// Converte um Int (I64) nativo para uma String alocada em C.
/// Retorna o ponteiro bruto que representa o tipo `Text` na Kata-Lang.
#[no_mangle]
pub extern "C" fn kata_rt_int_to_str(value: i64) -> *mut c_char {
    let s = format!("{}", value);
    let c_str = std::ffi::CString::new(s).unwrap();
    // into_raw transfere o ownership da memória para o ambiente C/Cranelift (Memory Leak intencional no protótipo sem GC)
    c_str.into_raw()
}

/// Concatena duas strings e retorna uma nova string alocada.
#[no_mangle]
pub extern "C" fn kata_rt_concat_text(a: *const c_char, b: *const c_char) -> *mut c_char {
    if a.is_null() || b.is_null() {
        return std::ffi::CString::new("(null)").unwrap().into_raw();
    }

    let a_str = unsafe { CStr::from_ptr(a).to_string_lossy() };
    let b_str = unsafe { CStr::from_ptr(b).to_string_lossy() };

    let result = format!("{}{}", a_str, b_str);
    std::ffi::CString::new(result).unwrap().into_raw()
}
