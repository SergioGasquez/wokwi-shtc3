use std::ffi::{c_void, CString};
use wokwi_chip_ll::{debugPrint, i2cInit, pinInit, I2CConfig, INPUT};

struct Chip {
    internal_address: Register,
    state: State,
}

enum State {
    ExpectingConnect,
    ExpectingReadByte1,
    ExpectingReadByte2,
    ExpectingReadByte3,
    ExpectingReadByte4,
    ExpectingReadByte5,
    ExpectingReadByte6,
    ExpectingWriteByte1,
    ExpectingWriteByte2,
    ExpectingWriteByte3,
}

#[derive(Clone, Copy)]
enum Register {
    Measurement = 0x66,
    ProductId = 0xC8,
    Uninitialized = 0xFF,
}

impl Register {
    fn from_address(address: u8) -> Register {
        match address {
            0x66 => Register::Measurement,
            0xC8 => Register::ProductId,
            _ => Register::Uninitialized,
        }
    }
}

const ADDRESS: u32 = 0x70;

const PRODUCT_ID_HI: u8 = 0x8;
const PRODUCT_ID_LO: u8 = 0x87;
const PRODUCT_ID_CRC: u8 = 0x5B;
const MEASUREMENT_TEMP_HI: u8 = 0x65;
const MEASUREMENT_TEMP_LO: u8 = 0xD5;
const MEASUREMENT_TEMP_CRC: u8 = 0x52;
const MEASUREMENT_HUM_HI: u8 = 0x5D;
const MEASUREMENT_HUM_LO: u8 = 0xD1;
const MEASUREMENT_HUM_CRC: u8 = 0x13;

static mut CHIP_VEC: Vec<Chip> = Vec::new();

#[no_mangle]
pub unsafe extern "C" fn chipInit() {
    debugPrint(CString::new("Initializing SHTC3").unwrap().into_raw());

    let chip = Chip {
        internal_address: Register::Uninitialized,
        state: State::ExpectingConnect,
    };
    CHIP_VEC.push(chip);

    let i2c_config: I2CConfig = I2CConfig {
        user_data: std::ptr::null::<c_void>(),
        address: ADDRESS,
        scl: pinInit(CString::new("SCL").unwrap().into_raw(), INPUT),
        sda: pinInit(CString::new("SDA").unwrap().into_raw(), INPUT),
        connect: on_i2c_connect as *const c_void,
        read: on_i2c_read as *const c_void,
        write: on_i2c_write as *const c_void,
        disconnect: on_i2c_disconnect as *const c_void,
    };
    i2cInit(&i2c_config);

    debugPrint(CString::new("Chip initialized!").unwrap().into_raw());
}

pub unsafe fn on_i2c_connect(user_ctx: *const c_void, address: u32, read: bool) -> bool {
    let msg: String = format!("on_i2c_connect: address: {}, read: {}", address, read);
    debugPrint(CString::new(msg).unwrap().into_raw());
    let chip: &mut Chip = &mut CHIP_VEC[user_ctx as usize];
    if read {
        chip.state = State::ExpectingReadByte1;
    } else {
        chip.state = State::ExpectingWriteByte1;
    }
    true
}

pub unsafe fn on_i2c_read(user_ctx: *const c_void) -> u8 {
    debugPrint(CString::new("on_i2c_read").unwrap().into_raw());
    let chip: &mut Chip = &mut CHIP_VEC[user_ctx as usize];

    match chip.state {
        State::ExpectingReadByte1 => match chip.internal_address {
            Register::ProductId => {
                chip.state = State::ExpectingReadByte2;
                return PRODUCT_ID_HI;
            }
            Register::Measurement => {
                chip.state = State::ExpectingReadByte2;
                return MEASUREMENT_TEMP_HI;
            }
            _ => {
                chip.state = State::ExpectingConnect;
            }
        },
        State::ExpectingReadByte2 => match chip.internal_address {
            Register::ProductId => {
                chip.state = State::ExpectingReadByte3;
                return PRODUCT_ID_LO;
            }
            Register::Measurement => {
                chip.state = State::ExpectingReadByte3;
                return MEASUREMENT_TEMP_LO;
            }
            _ => {
                chip.state = State::ExpectingConnect;
            }
        },
        State::ExpectingReadByte3 => match chip.internal_address {
            Register::ProductId => {
                chip.state = State::ExpectingReadByte4;
                return PRODUCT_ID_CRC;
            }
            Register::Measurement => {
                chip.state = State::ExpectingReadByte4;
                return MEASUREMENT_TEMP_CRC;
            }
            _ => {
                chip.state = State::ExpectingConnect;
            }
        },
        State::ExpectingReadByte4 => match chip.internal_address {
            Register::Measurement => {
                chip.state = State::ExpectingReadByte5;
                return MEASUREMENT_HUM_HI;
            }
            _ => {
                chip.state = State::ExpectingConnect;
            }
        },
        State::ExpectingReadByte5 => match chip.internal_address {
            Register::Measurement => {
                chip.state = State::ExpectingReadByte6;
                return MEASUREMENT_HUM_LO;
            }
            _ => {
                chip.state = State::ExpectingConnect;
            }
        },
        State::ExpectingReadByte6 => match chip.internal_address {
            Register::Measurement => {
                chip.state = State::ExpectingConnect;
                return MEASUREMENT_HUM_CRC;
            }
            _ => {
                chip.state = State::ExpectingConnect;
            }
        },
        _ => {
            chip.state = State::ExpectingConnect;
        }
    }
    0x0
}

pub unsafe fn on_i2c_write(user_ctx: *const c_void, data: u8) -> bool {
    let msg = format!("on_i2c_write: data: {}", data);
    debugPrint(CString::new(msg).unwrap().into_raw());

    let chip: &mut Chip = &mut CHIP_VEC[user_ctx as usize];

    chip.internal_address = Register::from_address(data);
    match chip.state {
        State::ExpectingWriteByte1 => {
            chip.state = State::ExpectingWriteByte2;
        }
        State::ExpectingWriteByte2 => {
            chip.state = State::ExpectingWriteByte3;
        }
        State::ExpectingWriteByte3 => {
            chip.state = State::ExpectingConnect;
        }
        _ => {
            chip.state = State::ExpectingConnect;
        }
    }
    true
}

pub unsafe fn on_i2c_disconnect(_user_ctx: *const c_void, _data: u8) {
    // Do nothing
}
