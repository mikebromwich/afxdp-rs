use std::convert::TryFrom;
use std::fmt;

use crate::buf::Buf;

/// BufMMap is the [Buf](crate::buf::Buf) implementation to be used with AF_XDP sockets (MMapArea, Umem, Socket).
#[derive(Debug)]
pub struct BufMmap<'a, T>
where
    T: std::default::Default,
{
    /// addr is is the address in the umem area (offset into area, in front of the headroom)
    pub(crate) addr: u64,
    /// len is the length of the buffer that is valid packet data
    pub(crate) len: u16,
    /// headroom is the number of bytes in the buffer prior to the packet data
    pub(crate) headroom: usize,
    /// data is the slice of u8 that contains the packet data
    pub(crate) data: &'a mut [u8],
    /// user is the user defined type
    pub(crate) user: T,
}

impl<T> Buf<T> for BufMmap<'_, T>
where
    T: std::default::Default,
{
    fn get_data(&self) -> &[u8] {
        &self.data[self.headroom..]
    }

    fn get_data_mut(&mut self) -> &mut [u8] {
        &mut self.data[self.headroom..]
    }

    fn get_data_with_headroom(&self) -> &[u8] {
        &self.data[0..]
    }

    fn get_data_with_headroom_mut(&mut self) -> &mut [u8] {
        &mut self.data[0..]
    }

    fn get_capacity(&self) -> u16 {
        u16::try_from(self.data.len() - self.headroom).unwrap()
    }

    fn get_len(&self) -> u16 {
        self.len
    }

    fn set_len(&mut self, len: u16) {
        if len > self.get_capacity() {
            panic!("len too large {} vs {}", len, self.get_capacity());
        }
        self.len = len;
    }

    fn set_headroom(&mut self, headroom: usize) {
        if headroom > self.get_capacity() as usize - self.headroom {
            panic!("headroom too large headroom {} vs {}", headroom, self.get_capacity() as usize + self.headroom);
        }
        self.headroom = headroom;
    }

    fn get_headroom(&self) -> usize {
        self.headroom
    }

    fn get_user(&self) -> &T {
        &self.user
    }

    fn get_user_mut(&mut self) -> &mut T {
        &mut self.user
    }
}

impl<'a, T> fmt::Display for BufMmap<'a, T>
where
    T: std::default::Default,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BufMMap addr={} len={} capacity={} headroom={} data={:?}",
            self.addr,
            self.len,
            self.get_capacity(),
            self.headroom,
            &(self.data[0]) as *const u8
        )
    }
}

impl<'a, T> Drop for BufMmap<'a, T>
where
    T: std::default::Default,
{
    fn drop(&mut self) {}
}

/*
#[derive(Debug)]
pub struct BufMmapConst<'a, T, const N: u16> where T: std::default::Default {
    pub addr: u64,
    pub data: &'a mut [u8],
    pub user: T,
}

impl<T, const N: u16> Buf<T> for BufMmapConst<'_, T, N> where T: std::default::Default {
    fn get_data(&self) -> &[u8] {
        &self.data[0..]
    }

    fn get_data_mut(&mut self) -> &mut [u8] {
        &mut self.data[0..]
    }

    fn get_len(&self) -> u16 {
        N
    }

    fn get_user(&self) -> &T {
        &self.user
    }

    fn get_user_mut(&mut self) -> &mut T {
        &mut self.user
    }
}

impl<'a, T, const N: u16> Drop for BufMmapConst<'a, T, N> where T: std::default::Default {
    fn drop(&mut self) {
        //todo!("bug");
    }
}
*/
