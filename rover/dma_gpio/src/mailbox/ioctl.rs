#![allow(non_snake_case)]
/*
* The original linux ioctl numbering scheme was just a general
* "anything goes" setup, where more or less random numbers were
* assigned.  Sorry, I was clueless when I started out on this.
*
* On the alpha, we'll try to clean it up a bit, using a more sane
* ioctl numbering, and also trying to be compatible with OSF/1 in
* the process. I'd like to clean it up for the i386 as well, but
* it's so painful recognizing both the new and the old numbers..
*
*  - word from linux kernel dev
*/

const _IOC_NRBITS: usize = 8;
const _IOC_TYPEBITS: usize = 8;
const _IOC_SIZEBITS: usize = 13;
const _IOC_DIRBITS: usize = 3;

const _IOC_NRMASK: usize = (1 << _IOC_NRBITS) - 1;
const _IOC_TYPEMASK: usize = (1 << _IOC_TYPEBITS) - 1;
const _IOC_SIZEMASK: usize = (1 << _IOC_SIZEBITS) - 1;
const _IOC_DIRMASK: usize = (1 << _IOC_DIRBITS) - 1;

const _IOC_NRSHIFT: usize = 0;
const _IOC_TYPESHIFT: usize = _IOC_NRSHIFT + _IOC_NRBITS;
const _IOC_SIZESHIFT: usize = _IOC_TYPESHIFT + _IOC_TYPEBITS;
const _IOC_DIRSHIFT: usize = _IOC_SIZESHIFT + _IOC_SIZEBITS;

/*
* Direction bits _IOC_NONE could be 0, but OSF/1 gives it a bit.
* And this turns out useful to catch old ioctl numbers in header
* files for us.
*/
const _IOC_NONE: usize = 1;
const _IOC_READ: usize = 2;
const _IOC_WRITE: usize = 4;


fn _IOC(dir: usize, i_type: usize, nr: usize, size: usize) -> usize {
    (dir << _IOC_DIRSHIFT) |
    (i_type << _IOC_TYPESHIFT) |
    (nr << _IOC_NRSHIFT) |
    (size << _IOC_SIZESHIFT)
}

// used to create numbers
// **one thing that is differnt here to the original ioctl.h is that
// the size parameter is not the pointer but
// the actual size of the pointer**
pub fn _IO(i_type: usize, nr: usize) -> usize {
    _IOC(_IOC_NONE, i_type, nr, 0)
}

pub fn _IOR(i_type: usize, nr: usize, size: usize) -> usize {
    _IOC(_IOC_READ, i_type, nr, size)
}

pub fn _IOW(i_type: usize, nr: usize, size: usize) -> usize {
    _IOC(_IOC_WRITE, i_type, nr, size)
}

pub fn _IOWR(i_type: usize, nr: usize, size: usize) -> usize {
    _IOC(_IOC_READ | _IOC_WRITE, i_type, nr, size)
}


// used to decode them..
pub fn _IOC_DIR(nr: usize) -> usize {
    (nr >> _IOC_DIRSHIFT) & _IOC_DIRMASK
}

pub fn _IOC_TYPE(nr: usize) -> usize {
    (nr >> _IOC_TYPESHIFT) & _IOC_TYPEMASK
}

pub fn _IOC_NR(nr: usize) -> usize {
    (nr >> _IOC_NRSHIFT) & _IOC_NRMASK
}

pub fn _IOC_SIZE(nr: usize) -> usize {
    (nr >> _IOC_SIZESHIFT) & _IOC_SIZEMASK
}



// ...and for the drivers/sound files
pub const IOC_IN: usize = _IOC_WRITE << _IOC_DIRSHIFT;
pub const IOC_OUT: usize = _IOC_READ << _IOC_DIRSHIFT;
pub const IOC_INOUT: usize = (_IOC_WRITE | _IOC_READ) << _IOC_DIRSHIFT;
pub const IOCSIZE_MASK: usize = _IOC_SIZEMASK << _IOC_SIZESHIFT;
pub const IOCSIZE_SHIFT: usize = _IOC_SIZESHIFT;

