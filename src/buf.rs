/// The Buf trait represents a packet buffer.
/// A trait is used so that an implementation can be provided that enables building and testing packet
/// pipelines without needing the AF_XDP infrastructure.
pub trait Buf<T>
where
    T: std::default::Default,
{
    /// Returns a reference to the u8 slice of the buffer excluding any preceding headroom
    fn get_data(&self) -> &[u8];

    /// Returns a mutable reference to the u8 slice of the buffer excluding any preceding headroom
    fn get_data_mut(&mut self) -> &mut [u8];

    /// Returns a reference to the u8 slice of the buffer including any preceding headroom
    fn get_data_with_headroom(&self) -> &[u8];

    /// Returns a mutable reference to the u8 slice of the buffer including any preceding headroom
    fn get_data_with_headroom_mut(&mut self) -> &mut [u8];

    /// Returns the total capacity of the buffer
    fn get_capacity(&self) -> u16;

    /// Returns the length of the portion of the buffer that contains packet data
    fn get_len(&self) -> u16;

    /// Sets the number of bytes in the buffer which precede the packet data
    fn set_headroom(&mut self, headroom: usize);

    /// Returns the number of bytes in the buffer which precede the packet data
    fn get_headroom(&self) -> usize;

    /// Sets the length of the portion of the buffer that is contains packet data
    fn set_len(&mut self, len: u16);

    /// Returns a reference to the embedded user struct
    fn get_user(&self) -> &T;

    /// Returns a mutable reference to the embeded user struct
    fn get_user_mut(&mut self) -> &mut T;
}

/*
pub trait BufConst<T, const N: usize> where T: std::default::Default {
    fn get_data(&self) -> &[u8; N];
    fn get_data_mut(&mut self) -> &mut [u8; N];

    fn get_len(&self) -> u16;

    fn get_user(&self) -> &T;
    fn get_user_mut(&mut self) -> &mut T;
}
*/
