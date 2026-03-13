pub mod channel;
pub mod memory;
pub mod scheduler;
pub mod io;
pub mod math;
pub mod collections;
pub mod closure;

#[cfg(test)]
mod tests {
    use super::memory::*;
    use super::channel::*;
    use std::sync::atomic::Ordering;
    use std::ptr;
    use super::math::*;
    use super::io::*;

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
        let pair = kata_rt_chan_create_queue(10);
        let mut arena = ThreadArena::new(1024);
        
        unsafe {
            let local_obj = arena.alloc(8, 1);
            let p1 = local_obj.add(KataHeader::SIZE);
            ptr::write(p1 as *mut u64, 777);

            let success = kata_rt_chan_send(pair.tx, local_obj);
            assert!(success);

            let header = local_obj as *mut KataHeader;
            assert_ne!((*header).forwarding_ptr, 0);
            let global_addr = (*header).forwarding_ptr as *mut u8;

            let recv_obj = kata_rt_chan_recv(pair.rx);
            assert_ne!(recv_obj, ptr::null_mut());
            assert_eq!(recv_obj, global_addr);

            kata_rt_release(recv_obj);
            let _ = Box::from_raw(pair.tx);
            let _ = Box::from_raw(pair.rx);
        }
    }
}
