use std::sync::Arc;
use std::{marker::PhantomData, u64};

use errno::errno;
use libc::{
    c_int, c_void, mmap, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_HUGETLB, MAP_PRIVATE, PROT_READ,
    PROT_WRITE,
};

use crate::buf_mmap::BufMmap;
use crate::AF_XDP_RESERVED;

/// A mapped memory area used to move packets between the kernel and userspace. One or more [Umem](crate::umem::Umem)
/// instances can share a Mmaparea.
#[derive(Debug)]
pub struct MmapArea<'a, T: std::default::Default + std::marker::Copy> {
    phantom: PhantomData<&'a T>,
    buf_num: usize,
    buf_len: usize,
    ptr: *mut c_void,
}
// MMapArea is not Send and Sync by default because of the raw pointer (ptr). According to the Rustonomicon,
// raw pointers are not Send/Sync as a 'lint'. I believe it is safe to mark MmapArea as Sync in this context.
// Note that the struct fields are private and never change.
// https://doc.rust-lang.org/nomicon/send-and-sync.html
// Note that we don't want to wrap MmapArea in an Mutex because we need the ptr to construct BufMMaps as buffers
// are passed back from the kernel. This happens at very high rates depending on the traffic.
unsafe impl<'a, T: std::default::Default + std::marker::Copy> Send for MmapArea<'a, T> {}
unsafe impl<'a, T: std::default::Default + std::marker::Copy> Sync for MmapArea<'a, T> {}

#[derive(Debug)]
pub enum MmapError {
    Failed,
}

/// Configuration options for MmapArea
#[derive(Default, Debug)]
pub struct MmapAreaOptions {
    /// If set to true, the mmap call is passed MAP_HUGETLB
    pub huge_tlb: bool,
}

impl<'a, T: std::default::Default + std::marker::Copy> MmapArea<'a, T> {
    /// Allocate a new memory mapped area based on the size and number of buffers
    ///
    /// # Arguments
    ///
    /// * buf_num: The number of buffers to allocate in the memory mapped area
    /// * buf_len: The length of each buffer
    /// * options: Configuration options
    pub fn new(
        buf_num: usize,
        buf_len: usize,
        options: MmapAreaOptions,
    ) -> Result<(Arc<MmapArea<'a, T>>, Vec<BufMmap<'a, T>>), MmapError> {
        let ptr: *mut c_void;
        let mut flags: c_int = MAP_PRIVATE | MAP_ANONYMOUS;

        if options.huge_tlb {
            flags |= MAP_HUGETLB
        }

        unsafe {
            ptr = mmap(
                std::ptr::null_mut::<c_void>(),
                buf_num * buf_len,
                PROT_READ | PROT_WRITE,
                flags,
                -1,
                0,
            );
        }

        if ptr == MAP_FAILED {
            return Err(MmapError::Failed);
        }

        let ma = Arc::new(MmapArea {
            buf_num,
            buf_len,
            ptr,
            phantom: PhantomData,
        });

        // Create the bufs
        let mut bufs = Vec::with_capacity(buf_num);
        let buf_len_available = buf_len as usize;

        for i in 0..buf_num {
            let buf: BufMmap<T>;
            unsafe {
                // addr is the offset into the memory mapped area
                let addr = (i * buf_len) as u64;
                let ptr = ma.ptr.offset(addr as isize);

                buf = BufMmap::<T> {
                    addr,
                    len: 0,
                    data: std::slice::from_raw_parts_mut(ptr as *mut u8, buf_len_available),
                    user: Default::default(),
                    headroom: AF_XDP_RESERVED.try_into().unwrap()
                };
            }

            bufs.push(buf);
        }

        Ok((ma, bufs))
    }

    /// Return the ptr to the memory mapped area.
    pub fn get_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Get the number of buffers in the memory mapped area.
    pub fn get_buf_num(&self) -> usize {
        self.buf_num
    }

    /// Get the size of the buffers in the memory mapped area.
    pub(crate) fn get_buf_len(&self) -> usize {
        self.buf_len
    }
}

impl<'a, T: std::default::Default + std::marker::Copy> Drop for MmapArea<'a, T> {
    fn drop(&mut self) {
        let r: c_int;

        // Don't try to unmap if the original map failed
        if self.ptr == MAP_FAILED {
            return;
        }

        unsafe {
            r = munmap(self.ptr, self.buf_num * self.buf_len);
        }

        if r != 0 {
            let errno = errno().0;
            println!("munmap failed errno: {}", errno);
        }
    }
}
#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::sync::Arc;

    use super::{MmapArea, MmapAreaOptions, MmapError};
    use crate::buf::Buf;
    use crate::buf_mmap::BufMmap;
    use crate::AF_XDP_RESERVED;

    #[derive(Default, Copy, Clone, Debug)]
    struct BufCustom {}

    /// Test that bufs ends up with the correct number of buffers and each is the correct length
    #[test]
    fn bufs_to_pool() {
        const BUF_NUM: usize = 1024;
        const BUF_LEN: usize = 2048;

        let options = MmapAreaOptions { huge_tlb: false };
        let r: Result<(Arc<MmapArea<BufCustom>>, Vec<BufMmap<BufCustom>>), MmapError> =
            MmapArea::new(BUF_NUM, BUF_LEN, options);

        let (area, bufs) = match r {
            Ok((area, bufs)) => (area, bufs),
            Err(err) => panic!("{:?}", err),
        };

        assert_eq!(area.buf_num, BUF_NUM);
        assert_eq!(area.buf_len, BUF_LEN);
        assert_eq!(bufs.len(), BUF_NUM);

        for buf in bufs {
            if buf.get_data().len() != BUF_LEN - AF_XDP_RESERVED as usize {
                panic!(
                    "expected buf len {} found {}",
                    BUF_LEN,
                    buf.get_data().len()
                );
            }
        }
    }

    // Test writing and reading multi-byte values at the start of each buf
    #[test]
    fn buf_values() {
        const BUF_NUM: usize = 1024;
        const BUF_LEN: usize = 2048;

        let options = MmapAreaOptions { huge_tlb: false };
        let r: Result<(Arc<MmapArea<BufCustom>>, Vec<BufMmap<BufCustom>>), MmapError> =
            MmapArea::new(BUF_NUM, BUF_LEN, options);

        let (area, mut bufs) = match r {
            Ok((area, buf_pool)) => (area, buf_pool),
            Err(err) => panic!("{:?}", err),
        };

        assert_eq!(area.buf_num, BUF_NUM);
        assert_eq!(area.buf_len, BUF_LEN);
        assert_eq!(bufs.len(), BUF_NUM);

        //
        // Write a value to each buf and then ensure we read the same values out
        //
        let base: u64 = 3983989832773837873;

        for (i, buf) in bufs.iter_mut().enumerate() {
            let val = i as u64 + base;
            let bytes = val.to_ne_bytes();
            buf.data[0] = bytes[0];
            buf.data[1] = bytes[1];
            buf.data[2] = bytes[2];
            buf.data[3] = bytes[3];
            buf.data[4] = bytes[4];
            buf.data[5] = bytes[5];
            buf.data[6] = bytes[6];
            buf.data[7] = bytes[7];
        }

        for (i, buf) in bufs.iter_mut().enumerate() {
            let val: u64 = i as u64 + base;

            let (int_bytes, _rest) = buf.data.split_at(std::mem::size_of::<u64>());
            let val2 = u64::from_ne_bytes(int_bytes.try_into().unwrap());

            assert_eq!(val, val2);
        }
    }

    // Test writing and reading from each byte of each buf
    #[test]
    fn buf_values2() {
        const BUF_NUM: usize = 256;
        const BUF_LEN: usize = 2048;

        let options = MmapAreaOptions { huge_tlb: false };
        let r: Result<(Arc<MmapArea<BufCustom>>, Vec<BufMmap<BufCustom>>), MmapError> =
            MmapArea::new(BUF_NUM, BUF_LEN, options);

        let (area, mut bufs) = match r {
            Ok((area, buf_pool)) => (area, buf_pool),
            Err(err) => panic!("{:?}", err),
        };

        assert_eq!(area.buf_num, BUF_NUM);
        assert_eq!(area.buf_len, BUF_LEN);
        assert_eq!(bufs.len(), BUF_NUM);

        //
        // Write the buf number into every byte of each buf
        //
        for (i, buf) in bufs.iter_mut().enumerate() {
            for j in 0..BUF_LEN - AF_XDP_RESERVED as usize {
                buf.data[j] = i as u8;
            }
        }

        //
        // Validate that the buf number is present in every byte of each buf
        //
        for (i, buf) in bufs.iter_mut().enumerate() {
            for j in 0..BUF_LEN - AF_XDP_RESERVED as usize {
                assert_eq!(buf.data[j], i as u8);
            }
        }
    }
}
