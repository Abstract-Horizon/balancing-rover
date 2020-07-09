#![allow(non_snake_case)]
#![allow(dead_code)]

//! Module for Board and BoardBuilder structs
//! 
//! Use [BoardBuilder](struct.BoardBuilder.html) struct to initialize the [Board](struct.Board.html) struct,
//! and use [Board](struct.Board.html) struct to manipulate GPIO Pins.

use crate::mailbox;

use libc;
use std::ptr;
use std::mem::size_of;
use std::ffi::CString;
use core::ffi::c_void;
use std::thread::sleep;
use std::time::Duration;
use std::io::{Error, ErrorKind};
use std::fs;
use volatile_register::RW;


/// = 32. The highest gpio we can address.
pub const MAX_CHANNELS: usize = 32;

/// = 9. Default number of channels.
pub const DEFAULT_NUM_CHANNELS: usize = 9;

/// [4, 17, 18, 27, 21, 22, 23, 24, 25]. 9 default GPIO pins on pi.
pub static DEFAULT_PINS: [u8; MAX_CHANNELS] = [
    4,              // P1-7
    17,             // P1-11
    18,             // P1-12
    27,             // P1-13
    21,             // P1-40
    22,             // P1-15
    23,             // P1-16
    24,             // P1-18
    25,             // P1-22
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 //empty possible channels
    ];

/// [6, 28, 29, 30, 31, 40, 45, 46, 47, 48, 49, 50, 51, 52, 53]. List of reserved GPIO pins
pub static BANNED_PINS: [u8; 15] = [
6,              // On Model B, it is in use for the Ethernet function
28,             // board ID and are connected to resistors R3 to R10 (only on Rev1.0 boards).
29,             // board ID and are connected to resistors R3 to R10 (only on Rev1.0 boards).
30,             // board ID and are connected to resistors R3 to R10 (only on Rev1.0 boards).
31,             // board ID and are connected to resistors R3 to R10 (only on Rev1.0 boards).
40,             // used by analogue audio
45,             // used by analogue audio
46,             // HDMI hotplug detect
47,             // 47 to 53 are used by the SD card interface.
48,
49,
50,
51,
52,
53,
];

const DEVFILE_MBOX: &str = "/dev/pi_gpio_mbox";
const DEVFILE_VCIO: &str = "/dev/vcio";

const PAGE_SIZE: usize = 4096;
const PAGE_SHIFT: usize = 12;

/// = 2000. Default period of the PWM signal.
pub const DEFAULT_CYCLE_TIME: usize = 2000;

/// = 10. Pulse width increment granularity.
/// 
/// Setting SAMPLE_DELAY too low will likely cause problems as the DMA controller
/// will use too much memory bandwidth.
/// 10 is a good value, though you might be ok setting it as low as 2.
pub const DEFAULT_SAMPLE_DELAY: usize = 10;

/// = 500. Default value for pwm div.
/// 
/// PWM runs at the frequency of 500 MHz.
/// So setting this as 500 will give us 500MHz/500 = 1 MHz.
/// You can change this configuration with [BoardBuilder::divide_pwm(mut self, divisor)](struct.BoardBuilder.html#method.divide_pwm: usize)
pub const DEFAULT_PWM_DIVISOR: usize = 500;

/// = DEFAULT_CYCLE_TIME/DEFAULT_SAMPLE_DELAY = 200. Number of samples.
pub const NUM_SAMPLES: usize = DEFAULT_CYCLE_TIME as usize/DEFAULT_SAMPLE_DELAY;

/// = NUM_SAMPLES * 2 = 400. Number of Control Blocks.
///
/// This is how much memory that will be allocated for control blocks.
/// Setting a different number for cycle time ([BoardBuilder::set_cycle_time](struct.BoardBuilder.html#method.set_cycle_time))
/// and setting a different number of sample delay ([BoardBuilder::set_sample_delay](struct.BoardBuilder.html#method.set_sample_delay))
/// will still allocate memory for 400 control blocks, but will only initialize (cycle_time/sample_delay) control blocks. 
pub const NUM_CBS: usize = NUM_SAMPLES*2;

const DMA_NO_WIDE_BURSTS: usize = 1<<26;
const DMA_WAIT_RESP: usize = 1<<3;
const DMA_D_DREQ: usize = 1<<6;
fn DMA_PER_MAP(x: usize) -> usize{
    x << 16
}
const DMA_END: usize = 1<<1;
const DMA_RESET: usize = 1<<31;
const DMA_INT: usize = 1<<2;

const DMA_CS: usize = 0x00/4;
const DMA_CONBLK_AD: usize = 0x04/4;
const DMA_DEBUG: usize = 0x20/4;

const GPIO_FSEL0: usize = 0x00/4;
const GPIO_SET0: usize = 0x1c/4;
const GPIO_CLR0: usize = 0x28/4;
const GPIO_LEV0: usize = 0x34/4;
const GPIO_PULLEN: usize = 0x94/4;
const GPIO_PULLCLK: usize = 0x98/4;

const GPIO_MODE_IN: usize = 0;
const GPIO_MODE_OUT: usize = 1;

const PWM_CTL: usize = 0x00/4;
const PWM_DMAC: usize = 0x08/4;
const PWM_RNG1: usize = 0x10/4;
const PWM_FIFO: usize = 0x18/4;

const PWMCLK_CNTL: usize = 40;
const PWMCLK_DIV: usize = 41;

const PWMCTL_MODE1: usize = 1<<1;
const PWMCTL_PWEN1: usize = 1<<0;
const PWMCTL_CLRF: usize = 1<<6;
const PWMCTL_USEF1: usize = 1<<5;

const PWMDMAC_ENAB: usize = 1<<31;
const PWMDMAC_THRSHLD: usize = (15<<8)|(15<<0);

const PCM_CS_A: usize = 0x00/4;
const PCM_FIFO_A: usize = 0x04/4;
const PCM_MODE_A: usize = 0x08/4;
const PCM_RXC_A: usize = 0x0c/4;
const PCM_TXC_A: usize = 0x10/4;
const PCM_DREQ_A: usize = 0x14/4;
const PCM_INTEN_A: usize = 0x18/4;
const PCM_INT_STC_A: usize = 0x1c/4;
const PCM_GRAY: usize = 0x20/4;

const PCMCLK_CNTL: usize = 38;
const PCMCLK_DIV: usize = 39;

/// Indicates using PWM
pub const DELAY_VIA_PWM: u8 = 0;

/// Indicates using PCM
pub const DELAY_VIA_PCM: u8 = 1;

/* New Board Revision format:
SRRR MMMM PPPP TTTT TTTT VVVV

S scheme (0=old, 1=new)
R RAM (0=256, 1=512, 2=1024)
M manufacturer (0='SONY',1='EGOMAN',2='EMBEST',3='UNKNOWN',4='EMBEST')
P processor (0=2835, 1=2836)
T type (0='A', 1='B', 2='A+', 3='B+', 4='Pi 2 B', 5='Alpha', 6='Compute Module')
V revision (0-15)
*/
const BOARD_REVISION_SCHEME_MASK: usize = 0x1 << 23;
const BOARD_REVISION_SCHEME_OLD: usize = 0x0 << 23;
const BOARD_REVISION_SCHEME_NEW: usize = 0x1 << 23;
const BOARD_REVISION_RAM_MASK: usize = 0x7 << 20;
const BOARD_REVISION_MANUFACTURER_MASK: usize = 0xF << 16;
const BOARD_REVISION_MANUFACTURER_SONY: usize = 0 << 16;
const BOARD_REVISION_MANUFACTURER_EGOMAN: usize = 1 << 16;
const BOARD_REVISION_MANUFACTURER_EMBEST: usize = 2 << 16;
const BOARD_REVISION_MANUFACTURER_UNKNOWN: usize = 3 << 16;
const BOARD_REVISION_MANUFACTURER_EMBEST2: usize = 4 << 16;
const BOARD_REVISION_PROCESSOR_MASK: usize = 0xF << 12;
const BOARD_REVISION_PROCESSOR_2835: usize = 0 << 12;
const BOARD_REVISION_PROCESSOR_2836: usize = 1 << 12;
const BOARD_REVISION_TYPE_MASK: usize = 0xFF << 4;
const BOARD_REVISION_TYPE_PI1_A: usize = 0 << 4;
const BOARD_REVISION_TYPE_PI1_B: usize = 1 << 4;
const BOARD_REVISION_TYPE_PI1_A_PLUS: usize = 2 << 4;
const BOARD_REVISION_TYPE_PI1_B_PLUS: usize = 3 << 4;
const BOARD_REVISION_TYPE_PI2_B: usize = 4 << 4;
const BOARD_REVISION_TYPE_ALPHA: usize = 5 << 4;
const BOARD_REVISION_TYPE_PI3_B: usize = 8 << 4;
const BOARD_REVISION_TYPE_PI3_BP: usize = 0xD << 4;
const BOARD_REVISION_TYPE_CM: usize = 6 << 4;
const BOARD_REVISION_TYPE_CM3: usize = 10 << 4;
const BOARD_REVISION_REV_MASK: usize = 0xF;

fn BUS_TO_PHYS(x: usize) -> usize {
    x & (!0xC0000000)
}


const DMA_CHAN_SIZE: usize = 0x100; /* size of register space for a single DMA channel */
const DMA_CHAN_MAX: usize = 14; // number of DMA Channels we have... actually, there are 15... but channel fifteen is mapped at a different DMA_BASE, so we leave that one alone
const DMA_CHAN_NUM: usize = 14; // the DMA Channel we are using, NOTE: DMA Ch 0 seems to be used by X... better not use it ;)
const PWM_BASE_OFFSET: usize = 0x0020c000;
const PWM_LEN: usize = 0x28;
const CLK_BASE_OFFSET: usize = 0x00101000;
const CLK_LEN: usize = 0xA8;
const GPIO_BASE_OFFSET: usize = 0x00200000;
const GPIO_LEN: usize = 0x100;
const PCM_BASE_OFFSET: usize = 0x00203000;
const PCM_LEN: usize = 0x24;

// DMA Control Block
struct DmaCbT {
    info: RW<usize>,
    src: RW<usize>,
    dst: RW<usize>,
    length: RW<usize>,
    stride: RW<usize>,
    next: RW<usize>,
    _pad: [usize; 2],
}

// DMA Controller
struct Ctl {
    sample: [RW<usize>; NUM_SAMPLES],
    cb: [DmaCbT; NUM_CBS],
}

// MailBox
struct Mbox {
    handle: i32,                // from mbox_open()
    mem_ref: usize,               // from mem_allox()
    bus_addr: usize,              // from mem_lock()
    virt_addr: *mut c_void         // from mapmem()
}

impl Mbox {
    fn new(handle: i32, mem_ref: usize, bus_addr: usize, virt_addr: *mut c_void) -> Self {
        Mbox{handle, mem_ref, bus_addr, virt_addr}
    }
}

/// Struct for initialzing [Board](struct.Board.html) and configuring the settings.
/// 
/// BoardBuilder is the only way to initialize Board struct.
/// You can configure different settings for DMA and PWM using this struct.
/// 
/// # Examples
/// ## Building with default settings
/// 
/// ```no_run
/// use dma_gpio::pi::BoardBuilder;
/// 
/// fn main() {
///     let mut board = BoardBuilder::new().build().unwrap();
///
///     ...
///     
/// }
/// 
/// ```
/// 
/// This example will enable 9 pins (4, 17, 18, 27, 21, 22, 23, 24, 25) for use.
/// 
/// PWM clock will run at 500MHz/500 = 1 MHz.
/// 
/// Each cycle will be 2000/1MHz = 2000 us.
/// 
/// Sample Delay will be 10/1MHz = 10 us.
/// 
/// This will create 2000/10 = 200 samples, and 200*2 = 400 Control Blocks.
/// 
/// This means that PWM can have 0.005 (0.5 %) increment from 0.00 (0 %) to 1.00 (100 %),
/// with each delay taking 10 us per sample.
/// 
/// Overall GPIO frequency will be 1/2000us = 500 Hz
/// 
/// ## Building with custom settings
/// 
/// ```no_run
/// use dma_gpio::pi::BoardBuilder;
/// 
/// fn main() {
///     let mut board = BoardBuilder::new()
///         .divide_pwm(50)
///         .set_cycle_time(400)
///         .set_sample_delay(2)
///         .build_with_pins(vec![21, 22]).unwrap();
///     
///     ...
///     
/// }
/// 
/// ```
/// 
/// This example will enable 2 pins (21, 22) for use.
/// 
/// PWM clock will run at 500MHz/50 = 10 MHz.
/// 
/// Each cycle will be 400/10MHz = 40 us.
/// 
/// Sample Delay will be 2/10MHz = 0.2 us.
/// 
/// This will create 400/2 = 200 samples, and 200*2 = 400 Control Blocks.
/// 
/// This means that PWM can have 0.005 (5 %) increment from 0.00 (0 %) to 1.00 (100 %),
/// with each delay taking 0.2 us per sample.
/// 
/// Theoratical GPIO frequency will be 1/40us = 25 KHz.
/// 
/// However, because the limiting speed of DMA is around ~1.6 MHz,
/// 
/// the actual frequency will be around 8 KHz with PWM (1.6 MHz/200 Samples).
pub struct BoardBuilder {
    known_pins: [u8; MAX_CHANNELS],
    num_channels: usize,

    delay_hw: u8,

    pwm_divisor: usize,
    cycle_time: usize,
    sample_delay: usize,
}

impl BoardBuilder {
    /// Creates new instance of BoardBuilder.
    pub fn new() -> Self {
        BoardBuilder{
            delay_hw: DELAY_VIA_PWM,

            known_pins: DEFAULT_PINS,
            num_channels: DEFAULT_NUM_CHANNELS,

            pwm_divisor: DEFAULT_PWM_DIVISOR,
            cycle_time: DEFAULT_CYCLE_TIME,
            sample_delay: DEFAULT_SAMPLE_DELAY,
        }
    }

    /// Builds and returns Result<[Board](struct.Board.html)>.
    /// 
    /// ## Example
    /// ```no_run
    /// ...
    /// 
    /// fn main() {
    ///     let mut board = BoardBuilder::new().build().unwrap();
    ///     
    ///     ...
    ///     
    /// }
    /// ```
    pub fn build(&self) -> Result<Board, Error> {
        Board::new(self.delay_hw, self.known_pins, self.num_channels, self.pwm_divisor, self.cycle_time, self.sample_delay)
    }

    /// Builds and returns Result<[Board](struct.Board.html)> with specific pins.
    /// 
    /// Be sure to look out for banned pins: [6, 28, 29, 30, 31, 40, 45, 46, 47, 48, 49, 50, 51, 52, 53]
    /// 
    /// ## Example
    /// ```no_run
    /// ...
    /// 
    /// fn main() {
    ///     let mut board = BoardBuilder::new().build_with_pins(vec![21, 22]).unwrap();
    ///     
    ///     ...
    ///     
    /// }
    /// ```
    pub fn build_with_pins(mut self, pins: Vec<u8>) -> Result<Board, Error> {
        let pins: Vec<u8> = pins.iter().filter(|&&pin| pin > 0).map(|&pin| pin).collect();
        let pins_len = pins.len();
        let mut temp_pins = [0; MAX_CHANNELS];
        if pins_len <= MAX_CHANNELS {
            for i in 0..pins_len {
                if pins[i] >= MAX_CHANNELS as u8 {
                    let error = format!("ERROR: {:} is an invalid gpio\n", pins[i]);
                    error!("{}", error);
                    return Err(Error::new(ErrorKind::Other, error))
                }else if is_banned_pin(pins[i]){
                    let error = format!("ERROR: {:} is a banned gpio\nBanned pins: {:?}", pins[i], BANNED_PINS);
                    error!("{}", error);
                    return Err(Error::new(ErrorKind::Other, error))
                }else{
                    temp_pins[i] = pins[i];
                }
            }
        }else {
            let error = format!("ERROR: number of pins {} exceeds max number of channels: {}\n", pins_len, MAX_CHANNELS);
            error!("{}", error);
            return Err(Error::new(ErrorKind::Other, error))
        }

        self.num_channels = pins_len;
        self.known_pins = temp_pins;
        self.build()
    }

    /// Use pcm instead of pwm for dma scheduling
    /// 
    /// ## Example
    /// ```no_run
    /// ...
    /// 
    /// fn main() {
    ///     let mut board = BoardBuilder::new().use_pcm().build().unwrap();
    ///     
    ///     ...
    ///     
    /// }
    /// ```
    pub fn use_pcm(mut self) -> Self {
        self.delay_hw = DELAY_VIA_PCM;
        self
    }

    /// Set value for PWM DIV.
    /// 
    /// See this [example](struct.BoardBuilder.html#building-with-custom-settings) for more details on how it works.
    /// ## Example
    /// value of 50 will give 500MHz/50 = 10 MHz frequency of PWM clock.
    /// ```no_run
    /// ...
    /// 
    /// fn main() {
    ///     let mut board = BoardBuilder::new().divide_pwm(50).build().unwrap();
    ///     
    ///     ...
    ///     
    /// }
    /// ```
    /// 
    pub fn divide_pwm(mut self, divisor: usize) -> Self {
        if divisor > 1000 {
            self.pwm_divisor = 1000;
        }else {
            self.pwm_divisor = divisor;
        }
        self
    }

    /// Set cycle time.
    /// 
    /// See this [example](struct.BoardBuilder.html#building-with-custom-settings) for more details on how it works.
    /// 
    /// ## Example
    /// ```no_run
    /// ...
    /// 
    /// fn main() {
    ///     let mut board = BoardBuilder::new()
    ///         .set_cycle_time(400)
    ///         .set_sample_delay(20)
    ///         .build().unwrap();
    ///     
    ///     ...
    ///     
    /// }
    /// ```
    pub fn set_cycle_time(mut self, units: usize) -> Self {
        if units < 200{
            self.cycle_time = 200;
        }else if units > 1000 {
            self.cycle_time = 1000;
        }else {
            self.cycle_time = units;
        }
        self
    }

    /// Set sample delay.
    /// 
    /// See this [example](struct.BoardBuilder.html#building-with-custom-settings) for more details on how it works.
    /// 
    /// ## Example
    /// ```no_run
    /// ...
    /// 
    /// fn main() {
    ///     let mut board = BoardBuilder::new()
    ///         .set_cycle_time(400)
    ///         .set_sample_delay(20)
    ///         .build().unwrap();
    ///     
    ///     ...
    ///     
    /// }
    /// ```
    pub fn set_sample_delay(mut self, units: usize) -> Self {
        if units == 0 {
            self.sample_delay = 1;
        }else if units > 100{
            self.sample_delay = 100;
        }else {
            self.sample_delay = units;
        }
        self
    }
}

/// Struct for dealing with GPIO Pins.
/// 
/// Board is initialized through [BoardBuilder](struct.BoardBuilder.html).
/// 
/// Note that you can only manipulate pins that are set from BoardBuilder,
/// 
/// so if the pin you want to access is not one of the default pins: [4, 17, 18, 27, 21, 22, 23, 24, 25], make sure to set it with [BoardBuilder::build_with_pins](struct.BoardBuilder.html#method.build_with_pins).
/// 
/// ## Example
/// This example uses pins [21, 22, 23],
/// 
/// sets pin 21 to 25%, pin 22 to 50%, and pin 23 to 75%,
/// 
/// then, after 1 second, releases pin 22,
/// 
/// then, after 1 second, release all pins.
/// 
/// ```no_run
/// use std::thread::sleep;
/// use std::time::Duration;
/// use dma_gpio::pi::BoardBuilder;
/// 
/// fn main() {
///     let mut board = BoardBuilder::new().build_with_pins(vec![21, 22, 23]).unwrap();
///     board.print_info();
///     
///     board.set_pwm(21, 0.25).unwrap();
///     board.set_pwm(22, 0.50).unwrap();
///     board.set_pwm(23, 0.75).unwrap();
///     
///     let sec = Duration::from_millis(1000);
///     sleep(millis);
///     
///     board.release_pwm(22).unwrap();
///     
///     sleep(millis);
///     
///     board.release_all_pwm().unwrap();
///     
/// }
/// 
/// ```
pub struct Board {
    pwm_divisor: usize,
    cycle_time: usize,
    sample_delay: usize,

    num_pages: usize,
    num_samples: usize,

    // pi version specific addresses
    dma_base: usize,

    _pwm_base: usize,
    pwm_phys_base: usize,
    
    _clk_base: usize,
    
    _gpio_base: usize,
    gpio_phys_base: usize,
    
    _pcm_base: usize,
    pcm_phys_base: usize,

    _dma_virt_base: *const [RW<usize>;DMA_CHAN_SIZE/4], // base address of all DMA Channels
    dma_reg: *const [RW<usize>; DMA_CHAN_SIZE/4], // pointer to the DMA Channel registers we are using
    pwm_reg: *const [RW<usize>; PWM_LEN/4],
    pcm_reg: *const [RW<usize>; PCM_LEN/4],
    clk_reg: *const [RW<usize>; CLK_LEN/4],
    gpio_reg: *const [RW<usize>; GPIO_LEN/4],

    known_pins: [u8; MAX_CHANNELS],
    num_channels: usize,
    channel_pwm: [f32; MAX_CHANNELS],

    // pin2gpio array is not setup as empty to avoid locking all GPIO
    // inputs as PWM, they are set on the fly by the pin param passed.
    pin2gpio: [u8;MAX_CHANNELS],

    mbox: Mbox,
    delay_hw: u8,

    invert_mode: bool,
}

impl Drop for Board {
    fn drop(&mut self) {
        self.terminate();
    }
}

impl Board {
    // open a char device file used for communicating with kernel mbox driver
    fn mbox_open() -> Result<i32, Error> {
        // try to use /dev/vcio first (kernel 4.1+)
        let dev_vcio =  CString::new(DEVFILE_VCIO).unwrap().into_bytes_with_nul();
        match unsafe { libc::open(dev_vcio.as_ptr() as *const u8, 0) }{
            fd if fd < 0 => {
                // initialize mbox
                let dev_mbox =  CString::new(DEVFILE_MBOX).unwrap().into_bytes_with_nul();
                let mbox_ptr = dev_mbox.as_ptr();
                match fs::remove_file(DEVFILE_MBOX){
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
                if unsafe { libc::mknod(mbox_ptr, libc::S_IFCHR | 0600, libc::makedev(mailbox::MAJOR_NUM as u32, 0)) } < 0 {
                    error!("failed to create mailbox device");
                    return Err(Error::new(ErrorKind::Other, "failed to create mailbox device"))
                }
                match unsafe{ libc::open(mbox_ptr, 0) }{
                    fdd if fdd < 0 => {
                        error!("can't open device file: {:?}", DEVFILE_MBOX);
                        Err(Error::new(ErrorKind::Other, format!("can't open device file: {:?}", DEVFILE_MBOX)))
                    },
                    fdd => Ok(fdd)
                }
            },
            fd => Ok(fd)
        }
    }

    fn mbox_close(file_desc: i32) -> Result<(), Error> {
        match unsafe {libc::close(file_desc) }{
            0 => Ok(()),
            _ => {
                error!("closing mbox failed.");
                Err(Error::new(ErrorKind::Other, "closing mbox failed."))
            },
        }
    }

    // determine which pi model we're running on
    fn get_model(mbox_board_rev: usize) -> Result<(usize, usize, usize), Error> {

        let board_model = if (mbox_board_rev & BOARD_REVISION_SCHEME_MASK) == BOARD_REVISION_SCHEME_NEW {
            match mbox_board_rev & BOARD_REVISION_TYPE_MASK {
                BOARD_REVISION_TYPE_PI2_B => 2,
                BOARD_REVISION_TYPE_PI3_B | BOARD_REVISION_TYPE_PI3_BP | BOARD_REVISION_TYPE_CM3 => 3,
                _ => 1,
            }
        }else {
            1
        };

        #[cfg(feature = "debug")]
        {
            trace!("This is Pi-{}", board_model);
        }

        return match board_model {
            1 => {
                let periph_virt_base = 0x20000000;
                let periph_phys_base = 0x7e000000;
                let mem_flag = mailbox::MEM_FLAG_L1_NONALLOCATING | mailbox::MEM_FLAG_ZERO;
                Ok((periph_virt_base, periph_phys_base, mem_flag))
            },
            2 | 3 => {
                let periph_virt_base = 0x3f000000;
                let periph_phys_base = 0x7e000000;
                let mem_flag = mailbox::MEM_FLAG_L1_NONALLOCATING | mailbox::MEM_FLAG_ZERO;
                Ok((periph_virt_base, periph_phys_base, mem_flag))
            },
            _ => {
                Err(Error::new(ErrorKind::Other, format!("Unable to detect Board Model from board revision: {:?}", mbox_board_rev)))
            },
        }
    }

    fn map_peripheral(base: usize, len: usize) -> Result<*mut c_void, Error> {
        let dev_mem =  CString::new("/dev/mem").unwrap().into_bytes_with_nul();
        let dmem_ptr = dev_mem.as_ptr();
        match unsafe { libc::open(dmem_ptr as *const u8, libc::O_RDWR | libc::O_SYNC)}{
            fd if fd < 0 => {
                let error = format!("dma_gpio: failed to open /dev/mem.");
                Err(Error::new(ErrorKind::Other, error))
            },
            fd => match unsafe{ libc::mmap(ptr::null_mut(), len, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, fd, base as i64) } {
                libc::MAP_FAILED => {
                    let error = format!("pi_gpio: Failed to map peripheral at {:#010x}.", base);
                    Err(Error::new(ErrorKind::Other, error))
                },
                vaddr => {
                    unsafe{ libc::close(fd)};
                    Ok(vaddr)
                }
            }
        }
    }

    fn new(delay_hw: u8, known_pins: [u8;MAX_CHANNELS], num_channels: usize, pwm_divisor: usize, cycle_time: usize, sample_delay: usize) -> Result<Self, Error> {
        let mut mbox_handle: i32 = match Board::mbox_open(){
            Ok(fd) => fd,
            Err(e) => {
                return Err(e)
            }
        };
        #[cfg(feature = "debug")]
        {
            trace!("mbox_handle: {:?}", mbox_handle);
        }

        let mbox_board_rev = match mailbox::get_board_revision(mbox_handle){
            Ok(rev) => rev,
            Err(e) => {
                return Err(Error::new(ErrorKind::Other, format!("could not get board revision: {:?}", e)))
            }
        };
        #[cfg(feature = "debug")]
        {
            trace!("MBox Board Revision: {:#010x}", mbox_board_rev);
        }


        let num_samples = cycle_time as usize/sample_delay;

        let num_pages: usize = (NUM_CBS * size_of::<DmaCbT>() as usize + NUM_SAMPLES * 4 + PAGE_SIZE - 1)>>PAGE_SHIFT;

        let (periph_virt_base, periph_phys_base, mem_flag) = match Board::get_model(mbox_board_rev){
            Ok(res) => res,
            Err(e) => {
                let error = format!("could not get the pi model: {:?}", e);
                return Err(Error::new(ErrorKind::Other, error))
            }
        };

        let dma_base = 0x00007000 + periph_virt_base;

        let _pwm_base = PWM_BASE_OFFSET + periph_virt_base;
        let pwm_phys_base = PWM_BASE_OFFSET + periph_phys_base;

        let _clk_base = CLK_BASE_OFFSET + periph_virt_base;

        let _gpio_base: usize = GPIO_BASE_OFFSET + periph_virt_base;
        let gpio_phys_base: usize = GPIO_BASE_OFFSET + periph_phys_base;

        let _pcm_base: usize = PCM_BASE_OFFSET + periph_virt_base;
        let pcm_phys_base: usize = PCM_BASE_OFFSET + periph_phys_base;
        

        #[cfg(feature = "debug")]
        {
            match mailbox::get_dma_channels(mbox_handle){
                Ok(channels) => {
                    trace!("DMA Channels Info: {:#010x}, using DMA Channel: {}\n", channels, DMA_CHAN_NUM);
                },
                Err(e) => return Err(e)
            };
        }

        /* map the registers for all DMA Channels */
        let _dma_virt_base = match Board::map_peripheral(dma_base, DMA_CHAN_SIZE * (DMA_CHAN_MAX + 1)){
            Ok(ptr) => ptr as *const [RW<usize>;DMA_CHAN_SIZE/4],
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("dma_virt_base: {:?}", _dma_virt_base);
        }

        /* set dma_reg to point to the DMA Channel we are using */
        let dma_reg = (_dma_virt_base as usize + DMA_CHAN_NUM * DMA_CHAN_SIZE) as *const [RW<usize>;DMA_CHAN_SIZE/4];
        #[cfg(feature = "debug")]
        {
            trace!("dma_reg_ptr: {:?}", dma_reg);
        }

        // let dma_reg = unsafe{ *dma_reg_ptr };
        let pwm_reg = match Board::map_peripheral(_pwm_base, PWM_LEN){
            Ok(ptr) => ptr as *const [RW<usize>;PWM_LEN/4],
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("pwm_reg: {:?}", pwm_reg);
        }

        let pcm_reg = match Board::map_peripheral(_pcm_base, PCM_LEN){
            Ok(ptr) => ptr as *const [RW<usize>;PCM_LEN/4],
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("pcm_reg: {:?}", pcm_reg);
        }

        let clk_reg = match Board::map_peripheral(_clk_base, CLK_LEN){
            Ok(ptr) => ptr as *const [RW<usize>;CLK_LEN/4],
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("clk_reg: {:?}", clk_reg);
        }

        let gpio_reg = match Board::map_peripheral(_gpio_base, GPIO_LEN){
            Ok(ptr) => ptr as *const [RW<usize>;GPIO_LEN/4],
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("gpio_reg: {:?}", gpio_reg);
        }

        /* Use the mailbox interface to the VC to ask for physical memory */
        let mbox_mem_ref = match mailbox::mem_alloc(mbox_handle, num_pages * PAGE_SIZE, PAGE_SIZE, mem_flag) {
            Ok(ret) => ret,
            Err(e) => return Err(e)
        };
        // TODO: How do we know that succeeded?
        #[cfg(feature = "debug")]
        {
            trace!("mem_ref: {:#010x}", mbox_mem_ref);
        }

        let mbox_bus_addr = match mailbox::mem_lock(mbox_handle, mbox_mem_ref) {
            Ok(ret) => ret,
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("bus_addr: {:#010x}", mbox_bus_addr);
        }

        let mbox_virt_addr = match mailbox::mapmem(BUS_TO_PHYS(mbox_bus_addr), num_pages * PAGE_SIZE){
            Ok(ret) => ret,
            Err(e) => return Err(e)
        };
        #[cfg(feature = "debug")]
        {
            trace!("virt_addr: {:#010x}\n", mbox_virt_addr);
        }

        if (mbox_virt_addr & (PAGE_SIZE - 1)) > 0 {
            return Err(Error::new(ErrorKind::Other, "pi-gpio: Virtual address is not page aligned."))
        }

        // we're done with mbox now
        match Board::mbox_close(mbox_handle){
            Ok(()) => (),
            Err(e) => return Err(e)
        }
        mbox_handle = -1;

        let mbox = Mbox::new(mbox_handle, mbox_mem_ref, mbox_bus_addr, mbox_virt_addr as *mut c_void);

        let mut board = Board{
            pwm_divisor,
            cycle_time,
            sample_delay,

            num_pages,
            num_samples,

            dma_base,

            _pwm_base,
            pwm_phys_base,

            _clk_base,

            _gpio_base,
            gpio_phys_base,

            _pcm_base,
            pcm_phys_base,

            _dma_virt_base,
            dma_reg,

            pwm_reg,
            pcm_reg,

            clk_reg,
            gpio_reg,

            known_pins,
            num_channels,
            pin2gpio: [0; MAX_CHANNELS],
            channel_pwm: [0.0; MAX_CHANNELS],

            mbox,

            delay_hw,
            invert_mode: false,
        };

        board.init_ctrl_data();
        board.init_hardware(pwm_divisor, sample_delay);
        board.init_pwm();

        Ok(board)
    }

    fn mem_virt_to_phys(&self, virt: *const usize) -> usize {
        let offset = virt as usize - self.mbox.virt_addr as usize;
        offset + self.mbox.bus_addr
    }

    // bus address of the ram is 0x40000000. With this binary-or, writes to the returned address will bypass the CPU (L1) cache, but not the L2 cache. 0xc0000000 should be the base address if L2 must also be bypassed. However, the DMA engine is aware of L2 cache - just not the L1 cache (source: http://en.wikibooks.org/wiki/Aros/Platforms/Arm_Raspberry_Pi_support#Framebuffer )
    fn virt_to_uncached_phys(&self, virt: *const usize) -> usize {
        self.mem_virt_to_phys(virt) | 0x40000000
    }

    fn init_ctrl_data(&self) {
        #[cfg(feature = "debug")]
        {
            trace!("Initializing DMA...\n");
        }

        let ctl_ptr = self.mbox.virt_addr as *const Ctl;

        let phys_gpclr0 = self.gpio_phys_base + 0x28;
        let phys_gpset0 = self.gpio_phys_base + 0x1c;
        let phys_fifo_addr = if self.delay_hw == DELAY_VIA_PWM {
            self.pwm_phys_base + 0x18
        }else {
            self.pcm_phys_base + 0x04
        };

        unsafe{
            let sample_ptr = &((*ctl_ptr).sample) as *const [RW<usize>; NUM_SAMPLES];
            libc::memset(sample_ptr as *mut c_void, 0, size_of::<[usize;NUM_SAMPLES]>());
        }

        // calculate a mask to turn off all the servos
        let mut mask = 0;
        for i in 0..self.num_channels {
            mask |= 1 << self.known_pins[i];
        }
        #[cfg(feature = "debug")]
        {
            trace!("mask: {:#010x}", mask);
        }
        unsafe{
            for i in 0..self.num_samples {
                (*ctl_ptr).sample[i].write(mask);
            }
        }

        /* Initialize all the DMA commands. They come in pairs.
        *  - 1st command copies a value from the sample memory to a destination
        *    address which can be either the gpclr0 register or the gpset0 register
        *  - 2nd command waits for a trigger from an external source (PWM or PCM)
        */
        let mut j = 0;
        let mut cbp;
        let cb_size = size_of::<DmaCbT>();
        unsafe{
            for i in 0..self.num_samples {
                // first DMA command
                cbp = &(*ctl_ptr).cb[j];
                cbp.info.write(DMA_NO_WIDE_BURSTS | DMA_WAIT_RESP);
                cbp.src.write(self.virt_to_uncached_phys((&((*ctl_ptr).sample[i]) as *const RW<usize>) as *const usize));
                cbp.dst.write(if self.invert_mode {
                    phys_gpset0
                }else {
                    phys_gpclr0
                });
                cbp.length.write(4);
                cbp.stride.write(0);
                cbp.next.write(self.virt_to_uncached_phys((cbp as *const DmaCbT as usize + cb_size) as *const usize));

                j += 1;
                cbp = &(*ctl_ptr).cb[j];
                cbp.info.write(if self.delay_hw == DELAY_VIA_PWM {
                    DMA_NO_WIDE_BURSTS | DMA_WAIT_RESP | DMA_D_DREQ | DMA_PER_MAP(5)
                }else {
                    DMA_NO_WIDE_BURSTS | DMA_WAIT_RESP | DMA_D_DREQ | DMA_PER_MAP(2)
                });
                cbp.src.write(self.virt_to_uncached_phys(ctl_ptr as *const usize)); // any data will do
                cbp.dst.write(phys_fifo_addr);
                cbp.length.write(4);
                cbp.stride.write(0);
                cbp.next.write(self.virt_to_uncached_phys((cbp as *const DmaCbT as usize + cb_size) as *const usize));

                j += 1;
            }
            (*ctl_ptr).cb[j - 1].next.write(self.virt_to_uncached_phys(&(*ctl_ptr).cb as *const DmaCbT as *const usize));
        }
    }

    fn init_hardware(&self, pwm_divisor: usize, sample_delay: usize) {
        #[cfg(feature = "debug")]
        {
            trace!("Initializing PWM/PCM HW...\n");
        }

        let ctl_ptr = self.mbox.virt_addr as *mut Ctl;

        unsafe {
            if self.delay_hw == DELAY_VIA_PWM {
                // Initialize PWM
                (*self.pwm_reg)[PWM_CTL].write(0);
                udelay(10);
                (*self.clk_reg)[PWMCLK_CNTL].write(0x5A000006); // Source=PLLD (500 MHz)
                udelay(100);
                (*self.clk_reg)[PWMCLK_DIV].write(0x5A000000 | (pwm_divisor << 12)); // set pwm div to 500, giving 1MHz
                udelay(100);
                (*self.clk_reg)[PWMCLK_CNTL].write(0x5A000016); // Source = PLLD and enable
                udelay(100);
                (*self.pwm_reg)[PWM_RNG1].write(sample_delay as usize);
                udelay(10);
                (*self.pwm_reg)[PWM_DMAC].write((PWMDMAC_ENAB | PWMDMAC_THRSHLD) as usize);
                udelay(10);
                (*self.pwm_reg)[PWM_CTL].write(PWMCTL_CLRF);
                udelay(10);
                (*self.pwm_reg)[PWM_CTL].write(PWMCTL_USEF1 | PWMCTL_PWEN1);
                udelay(10);
            }else {
                // Initialize PCM
                (*self.pcm_reg)[PCM_CS_A].write(1); // Disable Rx+Tx, Enable PCM block
                udelay(100);
                (*self.clk_reg)[PCMCLK_CNTL].write(0x5A000006); // Source=PLLD (500 MHz)
                udelay(100);
                (*self.clk_reg)[PCMCLK_DIV].write(0x5A000000 | (pwm_divisor << 12)); // set pcm div to 500, giving 1MHz
                udelay(100);
                (*self.clk_reg)[PCMCLK_CNTL].write(0x5A000016); // Source = PLLD and enable
                udelay(100);
                (*self.pcm_reg)[PCM_TXC_A].write(0<<31 | 1<<30 | 0<<20 | 0<<16); // 1 channel, 8 bits
                udelay(100);
                (*self.pcm_reg)[PCM_MODE_A].write((sample_delay - 1) << 10);
                udelay(100);
                (*self.pcm_reg)[PCM_CS_A].modify(|val| val | 1<<4 | 1<<3); // Clear FIFOs
                udelay(100);
                (*self.pcm_reg)[PCM_DREQ_A].write(64<<24 | 64<<8); // DMA Req when one slot is free?
                udelay(100);
                (*self.pcm_reg)[PCM_CS_A].modify(|val| val | 1<<9); // Enable DMA
                udelay(100);
            }

            // Initialize the DMA
            (*self.dma_reg)[DMA_CS].write(DMA_RESET);
            udelay(10);
            (*self.dma_reg)[DMA_CS].write(DMA_INT | DMA_END);
            (*self.dma_reg)[DMA_CONBLK_AD].write(self.virt_to_uncached_phys(&(*ctl_ptr).cb as *const DmaCbT as *const usize));
            (*self.dma_reg)[DMA_DEBUG].write(7); // clear debug error flags
            (*self.dma_reg)[DMA_CS].write(0x10880001); // go, mid priority, wait for outstanding writes
        }

        if self.delay_hw == DELAY_VIA_PCM {
            unsafe {
                (*self.pcm_reg)[PCM_CS_A].modify(|val| val | 1<<2)
            }; // Enable Tx
        }
    }

    fn init_pwm(&mut self) {
        #[cfg(feature = "debug")]
        {
            trace!("Initializing PWM...\n");
        }
        self.update_pwm();
    }
}

impl Board {

    fn gpio_set(&mut self, pin: u8) {
        unsafe {
            if self.invert_mode {
                (*self.gpio_reg)[GPIO_SET0].write(1 << pin);
            }else{
                (*self.gpio_reg)[GPIO_CLR0].write(1 << pin);
            }
        }
    }

    fn gpio_set_mode(&mut self, pin: usize, mode: usize) {
        let i = GPIO_FSEL0 + pin/10;
        unsafe {
            let mut fsel: usize = (*self.gpio_reg)[i].read();

            fsel &= !(7 << ((pin % 10) * 3));
            fsel |= mode << ((pin % 10) * 3);
            (*self.gpio_reg)[i].write(fsel);
        }
    }

    // Set the pin to a pin2gpio element so pi_gpio can write to it,
    // and set the width of the PWM pulse to the element with the same index
    // in channel_pwm array.
    fn set_pin2gpio(&mut self, pin: u8, width: f32) -> Result<(), Error> {
        if width >= 0.0 || width <= 1.0 {
            for i in 0..self.num_channels {
                if self.pin2gpio[i] == pin {
                    self.channel_pwm[i] = width;
                    return Ok(())
                }else if self.pin2gpio[i] == 0 {
                    self.pin2gpio[i] = pin;
                    self.gpio_set(pin);
                    self.gpio_set_mode(pin as usize, GPIO_MODE_OUT);
                    self.channel_pwm[i] = width;
                    return Ok(())
                }
            }
            Err(Error::new(ErrorKind::Other, format!("Pin {} is not one of the known pins", pin)))
        }else {
            Err(Error::new(ErrorKind::Other, format!("Width {} out of range.", width)))
        }
    }

    // Set each provided pin to one in pin2gpio
    fn set_pin(&mut self, pin: u8, width: f32) -> Result<(), Error> {
        if self.is_known_pin(pin) {
            self.set_pin2gpio(pin, width)
        }else{
            let err = format!("GPIO {:?} is not enabled for dma-gpio module", pin);
            Err(Error::new(ErrorKind::Other, err))
        }
    }

    /// Set GPIO pin's pwm width.
    pub fn set_pwm(&mut self, pin: u8, width: f32) -> Result<(), Error> {
        match self.set_pin(pin, width) {
            Ok(()) => self.update_pwm(),
            Err(e) => return Err(e)
        }
        Ok(())
    }

    /// Set all known GPIO pins' pwm width.
    pub fn set_all_pwm(&mut self, width: f32) -> Result<(), Error> {
        for i in 0..self.num_channels {
            match self.set_pin(self.known_pins[i], width) {
                Ok(()) => (),
                Err(e) => return Err(e)
            }
        }
        self.update_pwm();
        Ok(())
    }

    /// Invert all known GPIO pins' outputs.
    pub fn set_invert_mode(&mut self, mode: bool) {
        self.invert_mode = mode;
        self.update_pwm();
    }

    // To avoid storing the same pin 2 times after one pin has been released
    // we compact the pin2gpio array so all ON PWM pins are at the begining.
    fn compact_pin2gpio(&mut self) {
        let mut j = 0;
        let mut tmp_pin2gpio: [u8; MAX_CHANNELS] = [0; MAX_CHANNELS];
        let mut tmp_channel_pwm: [f32; MAX_CHANNELS] = [0.0; MAX_CHANNELS];

        for i in 0..self.num_channels {
            if self.pin2gpio[i] != 0 {
                tmp_pin2gpio[j] = self.pin2gpio[i];
                tmp_channel_pwm[j] = self.channel_pwm[i];
                j += 1;
            }
        }

        // Set the remaining slots in the arrays to 0, to disable them
        for i in 0..self.num_channels {
            self.pin2gpio[i] = tmp_pin2gpio[i];
            self.channel_pwm[i] = tmp_channel_pwm[i];
        }
        self.num_channels = j;
    }

    // Pins can be relesead after being setup as PWM pins by writing the release <pin>
    // command to the /dev/pi_gpio file. We make sure to compact the pin2gpio array
    // that contains currently working pwm pins.
    fn release_pin2gpio(&mut self, pin: u8) -> Result<(), Error> {
        for i in 0..self.num_channels {
            if self.pin2gpio[i] == pin {
                self.channel_pwm[i] = 0.0;
                self.pin2gpio[i] = 0;
                return Ok(())
            }
        }
        self.compact_pin2gpio();
        Err(Error::new(ErrorKind::Other, format!("Pin {} is not one of the known pins", pin)))
    }

    // Function make sure the pin we want to release is a valid pin, if it is
    // then calls release_pin2gpio to delete it from currently ON pins.
    fn release_pin(&mut self, pin: u8) -> Result<(), Error> {
        if self.is_known_pin(pin) {
            self.release_pin2gpio(pin)
        }else{
            let err = format!("GPIO {:?} is not enabled for dma-gpio module", pin);
            Err(Error::new(ErrorKind::Other, err))
        }
    }

    /// Releases GPIO pin.
    pub fn release_pwm(&mut self, pin: u8) -> Result<(), Error> {
        match self.release_pin(pin) {
            Ok(()) => self.update_pwm(),
            Err(e) => return Err(e)
        }
        Ok(())
    }

    /// Releases all GPIO pins.
    pub fn release_all_pwm(&mut self) -> Result<(), Error> {
        self.channel_pwm = [0.0; MAX_CHANNELS];
        self.update_pwm();
        self.num_channels = 0;
        self.pin2gpio = [0; MAX_CHANNELS];
        Ok(())
    }
    
    /*    
    What we need to do here is:
    First DMA command turns on the pins that are >0
    All the other packets turn off the pins that are not used

    For the cpb packets (The DMA control packet)
    -> cbp[0]->dst = gpset0: set   the pwms that are active
    -> cbp[]->dst = gpclr0: clear when the sample has a value

    For the samples     (The value that is written by the DMA command to cbp[n]->dst)
    -> dp[0] = mask of the pwms that are active
    -> dp[n] = mask of the pwm to stop at time n

    We dont really need to reset the cb->dst each time but I believe it helps a lot
    in code readability in case someone wants to generate more complex signals.
    */
    fn update_pwm(&self) {
        let phys_gpclr0: usize = self.gpio_phys_base + 0x28;
        let phys_gpset0: usize = self.gpio_phys_base + 0x1c;

        let ctl_ptr = self.mbox.virt_addr as *const Ctl;

        // first we turn on the channels that need to be on
        // take the first DMA Packet and set its target to start pulse
        unsafe {
            (*ctl_ptr).cb[0].dst.write(
                if self.invert_mode {
                    phys_gpclr0
                }else {
                    phys_gpset0
                });
        }

        // now create a mask of all the pins that should be on
        let mut mask = 0;
        for i in 0..self.num_channels {
            // check the pin2gpio pin has been set to avoid locking all of them as PWM.
            if (self.channel_pwm[i] > 0.0) && (self.pin2gpio[i] > 0) {
                mask |= 1 << self.pin2gpio[i];
            }
        }

        // and give that to the DMA controller to write
        unsafe {
            (*ctl_ptr).sample[0].write(mask);
        }

        // now we go through all the samples and turn the pins off when needed
        unsafe {
            for j in 1..self.num_samples {
                (*ctl_ptr).cb[j*2].dst.write(
                    if self.invert_mode {
                        phys_gpset0
                    }else {
                        phys_gpclr0
                    });
                mask = 0;
                for i in 0..self.num_channels {
                    // check the pin2gpio pin has been set to avoid locking all of them as PWM.
                    if self.pin2gpio[i] > 0 && (j as f32/self.num_samples as f32 > self.channel_pwm[i]) {
                        mask |= 1 << self.pin2gpio[i];
                    }
                }
                (*ctl_ptr).sample[j].write(mask);
            }
        }
    }


    /// Check if the pin provided is found in the list of known pins set with [BoardBuilder::build_with_pins](struct.BoardBuilder.html#method.build_with_pins).
    pub fn is_known_pin(&self, pin: u8) -> bool {
        for i in 0..MAX_CHANNELS {
            if self.known_pins[i] == pin {
                return true
            }
        }
        false
    }

    /// Check if the pin provided is found in the list of BANNED pins.
    pub fn is_banned_pin(&self, pin: u8) -> bool {
        for i in 0..BANNED_PINS.len() {
            if BANNED_PINS[i] == pin {
                return true
            }
        }
        false
    }

    /// Sets all GPIO pins' pwm width to 0.0, and frees the memory used for the process.
    /// 
    /// Board already implements Drop trait that calls this method,
    /// so you won't ever have to call this method.
    pub fn terminate(&mut self) {
        let mut has_error = false;

        #[cfg(feature = "debug")]
        {
            trace!("Resetting DMA...");
        }
        if (self.dma_reg as usize > 0) && (self.mbox.virt_addr as usize > 0) {
            for i in 0..self.num_channels {
                self.channel_pwm[i] = 0.0;
            }
            self.update_pwm();
            udelay(DEFAULT_CYCLE_TIME as u64);
            unsafe {(*self.dma_reg)[DMA_CS].write(DMA_RESET)};
            udelay(10);
        }


        #[cfg(feature = "debug")]
        {
            trace!("Freeing mbox memory...");
        }
        if !self.mbox.virt_addr.is_null() {
            match mailbox::unmapmem(self.mbox.virt_addr, self.num_pages * PAGE_SIZE){
                Ok(_) => (),
                Err(e) => {
                    error!("{:?}", e);
                    has_error = true;
                },
            }
            if self.mbox.handle <= 2 {
                match Board::mbox_open(){
                    Ok(mbox_handle) => {
                        match mailbox::mem_unlock(mbox_handle, self.mbox.mem_ref){
                            Ok(_) => (),
                            Err(e) => {
                                error!("{:?}", e);
                                has_error = true;
                            }
                        }
                        match mailbox::mem_free(mbox_handle, self.mbox.mem_ref) {
                            Ok(_) => (),
                            Err(e) => {
                                error!("{:?}", e);
                                has_error = true;
                            }
                        }
                        match Board::mbox_close(mbox_handle) {
                            Ok(()) => (),
                            Err(_) => {
                                error!("file close error");
                                has_error = true
                            }
                        }
                    },
                    Err(e) => {
                        error!("{:?}", e);
                        has_error = true;
                    },
                }
            }
        }

        if has_error {
            println!("unsuccessfully terminated.");
        }else{
            println!("dma_gpio stopped.");
        }
    }

    /// print info about the hardware: PWM or PCM, Number of channels, Pins being used, PWM Frequency, PWM steps, Maximum Period, Minimum Period, and DMA Base Address.
    pub fn print_info(&self) {
        println!("Using hardware:\t\t\t{:}", if self.delay_hw == DELAY_VIA_PWM {"PWM"} else{"PCM"});
        println!("Number of channels:\t\t{}", self.num_channels);

        #[allow(array_into_iter)]
        let print_pins: Vec<&u8> = self.known_pins.into_iter().filter(|&&pin| pin > 0).collect();
        println!("Pins:\t\t\t\t{:?}", print_pins);
        println!("PWM frequency:\t\t\t{} Hz", 500000000.0/(self.pwm_divisor * self.cycle_time) as f64);
        println!("PWM steps:\t\t\t{}", self.num_samples);
        println!("Maximum period (100 %):\t{} us", ((self.cycle_time * self.pwm_divisor) as f64/500.0));
        println!("Minimum period ({:3} %):\t{} us", 100.0*self.sample_delay as f64 / self.cycle_time as f64, (self.sample_delay * self.pwm_divisor) as f64/500.0);
        println!("DMA Base:\t\t\t{:#010x}", self.dma_base);
    }

    /// This method is only available when 'debug' feature is on.
    /// 
    /// Print out all informations about the control blocks, PWM, Clock, GPIO and DMA.
    #[cfg(feature = "debug")]
    pub fn debug_dump_hw(&self) {
        trace!("\n");
        trace!("pwm_reg: {:?}\n", self.pwm_reg);

        let ctl_ptr = self.mbox.virt_addr as *const Ctl;
        let mut cbp;

        for i in 0..self.num_samples {
            unsafe{
                cbp = &(*ctl_ptr).cb[i];
            }
            trace!("DMA Control Block: #{} @{:?}", i, cbp as *const DmaCbT);
            trace!("info:\t{:#010x}", cbp.info.read());
            trace!("src:\t{:#010x}", cbp.src.read());
            trace!("dst:\t{:#010x}", cbp.dst.read());
            trace!("length:\t{:#010x}", cbp.length.read());
            trace!("stride:\t{:#010x}", cbp.stride.read());
            trace!("next:\t{:#010x}\n", cbp.next.read());
        }

        trace!("PWM_BASE:\t{:#010x}", self._pwm_base);
        trace!("PWM_REG:\t{:?}", self.pwm_reg);
        unsafe {
            for i in 0..(PWM_LEN/4) {
                trace!("{:#04X}: {:#010x} {:#010x}", i, self.pwm_reg as usize + 4*i, (*self.pwm_reg)[i].read());
            }
        }
        trace!("\n");
        trace!("CLK_BASE: {:#010x}", self._clk_base);
        trace!("PWMCLK_CNTL: {:#010x}", PWMCLK_CNTL);
        trace!("clk_reg[PWMCLK_CNTL]: {:#010x}", self.clk_reg as usize + 4*PWMCLK_CNTL);
        trace!("PWMCLK_DIV: {:#010x}", PWMCLK_DIV);
        trace!("clk_reg: {:?}", self.clk_reg);
        trace!("virt_to_phys(clk_reg): {:#010x}", self.virt_to_uncached_phys(self.clk_reg as *const usize));
        unsafe {
            for i in 0..(CLK_LEN/4) {
                trace!("{:#04X}: {:#010x} {:#010x}", i, self.clk_reg as usize + 4*i, (*self.clk_reg)[i].read());
            }
        }
        trace!("\n");
        trace!("DMA_BASE: {:#010x}", self.dma_base);
        trace!("dma_virt_base: {:?}", self._dma_virt_base);
        trace!("dma_reg: {:?}", self.dma_reg);
        trace!("virt_to_phys(dma_reg): {:#010x}", self.virt_to_uncached_phys(self.dma_reg as *const usize));
        unsafe {
            for i in 0..(DMA_CHAN_SIZE/4) {
                trace!("{:#04X}: {:#010x} {:#010x}", i, self.dma_reg as usize + i*4, (*self.dma_reg)[i].read());
            }
        }
        trace!("\n");
        trace!("GPIO_BASE: {:#010x}", self._gpio_base);
        trace!("gpio_reg: {:?}", self.gpio_reg);
        trace!("virt_to_phys(gpio_reg): {:#010x}", self.virt_to_uncached_phys(self.gpio_reg as *const usize));
        unsafe {
            for i in 0..(GPIO_LEN/4) {
                trace!("{:#04X}: {:#010x} {:#010x}", i, self.gpio_reg as usize + i*4, (*self.gpio_reg)[i].read());
            }
        }
    }

    /// This method is only available when 'debug' feature is on.
    /// 
    /// Print out info about samples' outputs.
    #[cfg(feature = "debug")]
    pub fn debug_dump_samples(&self) {
        let ctl_ptr = self.mbox.virt_addr as *const Ctl;

        unsafe{
            for i in 0..self.num_samples {
                trace!("#{} @{:#010x}", i, (*ctl_ptr).sample[i].read());
            }
        }
    }
}

/// delay for # us seconds.
pub fn udelay(us: u64) {
    let nanos = Duration::from_nanos(us*1000);
    sleep(nanos);
}

/// Check if the pin provided is found in the list of BANNED pins.
pub fn is_banned_pin(pin: u8) -> bool {
    for i in 0..BANNED_PINS.len() {
        if BANNED_PINS[i] == pin {
            return true
        }
    }
    false
}

