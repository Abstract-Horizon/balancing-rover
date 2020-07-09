#![allow(non_snake_case)]

//! Mailbox Module

use core::ffi::c_void;
use std::ffi::CString;
use std::io::{Error, ErrorKind};
use std::mem::size_of;
// use std::fs::OpenOptions;
// use std::{io, ptr};
use libc;

pub mod ioctl;

/* from https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface */
pub const MEM_FLAG_DISCARDABLE: usize = 1 << 0; /* can be resized to 0 at any time. Use for cached data */
pub const MEM_FLAG_NORMAL: usize = 0 << 2; /* normal allocating alias. Don't use from ARM */
pub const MEM_FLAG_DIRECT: usize = 1 << 2; /* 0xC alias uncached */
pub const MEM_FLAG_COHERENT: usize = 2 << 2; /* 0x8 alias. Non-allocating in L2 but coherent */
pub const MEM_FLAG_L1_NONALLOCATING: usize = MEM_FLAG_DIRECT | MEM_FLAG_COHERENT; /* Allocating in L2 */
pub const MEM_FLAG_ZERO: usize = 1 << 4;  /* initialise buffer to all zeros */
pub const MEM_FLAG_NO_INIT: usize = 1 << 5; /* don't initialise (default is initialise to all ones */
pub const MEM_FLAG_HINT_PERMALOCK: usize = 1 << 6; /* Likely to be locked for long periods of time. */

// pointer size is 4 bytes for 32-bit machine a.k.a. rpi
const PTR_SIZE: usize = 4;
pub const MAJOR_NUM: usize = 100;

const PAGE_SIZE: usize = PTR_SIZE*1024;

pub fn mapmem(base: usize, size: usize) -> Result<usize, Error> {
    let offset = base % PAGE_SIZE;

    let base = base - offset;

    // open /dev/mem
    let dev_mem =  CString::new("/dev/mem").unwrap().into_bytes_with_nul();
    let mem_fd = match unsafe { libc::open(dev_mem.as_ptr(), libc::O_RDWR|libc::O_SYNC) }{
        fd if fd < 0 => {
            error!("can't open /dev/mem\nThis program should be run as root. Try prefixing command with: sudo");
            return Err(Error::new(ErrorKind::PermissionDenied, "can't open /dev/mem\nThis program should be run as root. Try prefixing command with: sudo"))
        },
        fd => fd
    };

    let mem = unsafe {
        libc::mmap(
            0 as *mut c_void,
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            mem_fd,
            base as i32
            ) as usize
    };

    #[cfg(feature = "debug")]
    {
        trace!("base={:#010x}, mem={:#010x}", base, mem);
    }

    if mem == libc::MAP_FAILED as usize {
        error!("map error {:?}\n", mem);
        return Err(Error::new(ErrorKind::Other, format!("map error {:?}\n", mem)))
    }

    unsafe {
        libc::close(mem_fd);
    };

    Ok(mem + offset)
}

pub fn unmapmem(addr: *mut c_void, size: usize) -> Result<(), Error> {
    match unsafe{ libc::munmap(addr, size) } {
        0 => Ok(()),
        s => {
            error!("munmap error: {:?}\n", s);
            Err(Error::new(ErrorKind::Other, format!("munmap error: {:?}\n", s)))
        },
    }
}

pub fn mbox_property(file_desc: i32, buf: &mut [usize; 32], _len: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("Mbox request:");
        for i in 0.._len {
            trace!("{:#04x}: {:#010x}", i*size_of::<u8>(), buf[i]);
        }
        trace!("\n");
    }

    // the third parameter is the size of a pointer
    // in c code, this is passed in as "char *"
    // so sizeof(char *) is 4 for a 32-bit machine a.k.a. rpi
    let IOCTL_MBOX_PROPERTY: usize = ioctl::_IOWR(MAJOR_NUM, 0, PTR_SIZE);
    let ret_val = match unsafe{ libc::ioctl(file_desc, IOCTL_MBOX_PROPERTY as u32, buf.as_mut_ptr() as *mut c_void) }{
        x if x < 0 => {
            error!("ioctl_set_msg failed: {:?}", x);
            return Err(Error::new(ErrorKind::Other, format!("ioctl_set_msg failed: {:?}", x)))
        },
        x => x as usize
    };

    #[cfg(feature = "debug")]
    {
        trace!("Mbox responses:");
        for i in 0.._len {
            trace!("{:#04x}: {:#010x}", i*size_of::<u8>(), buf[i]);
        }
        trace!("\n");
    }

    Ok(ret_val)
}

pub fn mem_alloc(file_desc: i32, size: usize, align: usize, flags: usize) -> Result<usize, Error> {
    let mut p: [usize;32] = [0; 32];
    #[cfg(feature = "debug")]
    {
        trace!("mem_alloc");
        trace!("Requesting {} bytes", size);
        trace!("Alignment {} bytes", align);
        trace!("mem_alloc flags: {:#010x} \n", flags);
    }

    p[1] = 0x00000000; // process request

    p[2] = 0x3000c; // (the tag id)
    p[3] = 12; // (size of the buffer)
    p[4] = 12; // (size of the data)
    p[5] = size; // (num bytes? or pages?)
    p[6] = align; // (alignment)
    p[7] = flags; // (MEM_FLAG_L!_NOMALLOCATING)
    p[8] = 0x00000000; // end tag

    p[0] = 9*size_of::<usize>();
    match mbox_property(file_desc, &mut p, 9){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn mem_free(file_desc: i32, handle: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("mem_free");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x3000f; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 4; // (size of the data)
    p[5] = handle;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();
    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn mem_lock(file_desc: i32, handle: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("mem_lock");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x3000d; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 4; // (size of the data)
    p[5] = handle;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();
    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn mem_unlock(file_desc: i32, handle: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("mem_unlock");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x3000e; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 4; // (size of the data)
    p[5] = handle;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();
    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn execute_code(file_desc: i32, code: usize, r0: usize, r1: usize, r2: usize, r3: usize, r4: usize, r5: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("execute_code");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x30010; // (the tag id)
    p[3] = 28; // (size of the buffer)
    p[4] = 28; // (size of the data)
    p[5] = code;
    p[6] = r0;
    p[7] = r1;
    p[8] = r2;
    p[9] = r3;
    p[10] = r4;
    p[11] = r5;
    p[12] = 0x00000000; // end tag

    p[0] = 13*size_of::<usize>();
    match mbox_property(file_desc, &mut p, 13){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn qpu_enable(file_desc: i32, enable: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("qpu_enable");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x30012; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 4; // (size of the data)
    p[5] = enable;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();

    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn execute_qpu(file_desc: i32, num_qpus: usize, control: usize, noflush: usize, timeout: usize) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("execute_qpu");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x30011; // (the tag id)
    p[3] = 16; // (size of the buffer)
    p[4] = 16; // (size of the data)
    p[5] = num_qpus;
    p[6] = control;
    p[7] = noflush;
    p[8] = timeout; // ms

    p[9] = 0x00000000; // end tag

    p[0] = 10*size_of::<usize>();

    match mbox_property(file_desc, &mut p, 10){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn get_firmware_revision(file_desc: i32) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("get_firmware_revision");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x10000; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 0; // (size of the data)
    p[5] = 0;

    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();

    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn get_board_model(file_desc: i32) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("get_board_model");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x10001; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 0; // (size of the data)
    p[5] = 0;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();

    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn get_board_revision(file_desc: i32) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("get_board_revision");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x10002; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 0; // (size of the data)
    p[5] = 0;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();

    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

pub fn get_dma_channels(file_desc: i32) -> Result<usize, Error> {
    #[cfg(feature = "debug")]
    {
        trace!("get_dma_channels");
    }
    let mut p: [usize;32] = [0; 32];

    p[1] = 0x00000000; // process request

    p[2] = 0x60001; // (the tag id)
    p[3] = 4; // (size of the buffer)
    p[4] = 0; // (size of the data)
    p[5] = 0;
    p[6] = 0x00000000; // end tag

    p[0] = 7*size_of::<usize>();

    match mbox_property(file_desc, &mut p, 7){
        Ok(_) => Ok(p[5]),
        Err(e) => Err(e),
    }
}

