use super::{Vec, prelude::*};

pub struct String {
    vec: Vec<u8>,
}

impl String {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn push(&mut self, c: char) {
        let mut int = c as u32;

        for _ in 0..c.len_utf8() {
            self.vec.push(int as u8);
            int >>= 8;
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        let mut int = self.vec.pop()? as u32;

        for _ in 0..3 {
            if let Some(c) = char::from_u32(int) {
                return Some(c);
            }

            int <<= 8;
            int |= self.vec.pop().unwrap() as u32;
        }

        let c = char::from_u32(int);
        assert!(c.is_some());
        c
    }

    pub fn from_str(string: &str) -> Self {
        let mut vec = Vec::with_capacity(string.len());

        for byte in string.as_bytes() {
            vec.push(*byte);
        }

        Self { vec }
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.vec) }
    }
}

impl AsMut<str> for String {
    fn as_mut(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(&mut self.vec) }
    }
}
