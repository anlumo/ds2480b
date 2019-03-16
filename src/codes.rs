pub enum Command {
    Reset                 = 0xC1,
    Pullup                = 0x3B,
    DataMode              = 0xE1,
    CommandMode           = 0xE3,
    SearchAccelleratorOn  = 0xB1,
    SearchAccelleratorOff = 0xA1,
    ConvertT              = 0x44,
    PullupArm             = 0xEF,
    PullupDisarm          = 0xED,
    PulseTerminate        = 0xF1,
    ReadScratchpad        = 0xBE,
    SkipROM               = 0xCC,
    MatchROM              = 0x55,
    SearchROM             = 0xF0,
    SearchAlarm           = 0xEC,
    Comm                  = 0x81,
    Config                = 0x01,
}

pub enum FunctionSelect {
    Bit       = 0x00,
    SearchOn  = 0x30,
    SearchOff = 0x20,
    Reset     = 0x40,
    Chmod     = 0x60,
}

pub enum SpeedSelect {
    Standard  = 0x00,
    Flex      = 0x04,
    Overdrive = 0x08,
    Pulse     = 0x0C,
}

pub enum ParameterSelect {
    ParmRead         = 0x00,
    Slew             = 0x10,
    Pulse12V         = 0x20,
    Pulse5V          = 0x30,
    Write1Low        = 0x40,
    SampleOffset     = 0x50,
    ActivePullupTime = 0x60,
    Baudrate         = 0x70,
}

pub enum SlewRate {
    Slew15Vus   = 0x00,
    Slew2p2Vus  = 0x02,
    Slew1p65Vus = 0x04,
    Slew1p37Vus = 0x06,
    Slew1p1Vus  = 0x08,
    Slew0p83Vus = 0x0A,
    Slew0p7Vus  = 0x0C,
    Slew0p55Vus = 0x0E,
}

pub enum Write1LowTime {
    Write8us  = 0x00,
    Write9us  = 0x02,
    Write10us = 0x04,
    Write11us = 0x06,
    Write12us = 0x08,
    Write13us = 0x0A,
    Write14us = 0x0C,
    Write15us = 0x0E,
}

pub enum SampleOffset {
    SampOff3us  = 0x00,
    SampOff4us  = 0x02,
    SampOff5us  = 0x04,
    SampOff6us  = 0x06,
    SampOff7us  = 0x08,
    SampOff8us  = 0x0A,
    SampOff9us  = 0x0C,
    SampOff10us = 0x0E,
}

pub enum PulseTime {
    Pulse32us     = 0x00,
    Pulse64us     = 0x02,
    Pulse128us    = 0x04,
    Pulse256us    = 0x06,
    Pulse512us    = 0x08,
    Pulse1024us   = 0x0A,
    Pulse2048us   = 0x0C,
    PulseInfinite = 0x0E,
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Level {
    Normal    = 0x00,
    Overdrive = 0x01,
    Strong5   = 0x02,
    Program   = 0x04,
    Break     = 0x08,
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Mode {
    Data    = 0x00,
    Command = 0x02,
}

pub enum BitPolarity {
    One = 0x10,
    Zero = 0x00,
}

pub mod reset_byte {
    pub const CHIPID_MASK  : u8 = 0x1C;
    pub const RESET_MASK   : u8 = 0x03;
    pub const ONEWIRESHORT : u8 = 0x00;
    pub const PRESENCE     : u8 = 0x01;
    pub const ALARMPRESENCE: u8 = 0x02;
    pub const NOPRESENCE   : u8 = 0x03;
}

const DSCRC_TABLE: [u8; 256] = [
    0, 94, 188, 226, 97, 63, 221, 131, 194, 156, 126, 32, 163, 253, 31, 65,
    157, 195, 33, 127, 252, 162, 64, 30, 95, 1, 227, 189, 62, 96, 130, 220,
    35, 125, 159, 193, 66, 28, 254, 160, 225, 191, 93, 3, 128, 222, 60, 98,
    190, 224, 2, 92, 223, 129, 99, 61, 124, 34, 192, 158, 29, 67, 161, 255,
    70, 24, 250, 164, 39, 121, 155, 197, 132, 218, 56, 102, 229, 187, 89, 7,
    219, 133, 103, 57, 186, 228, 6, 88, 25, 71, 165, 251, 120, 38, 196, 154,
    101, 59, 217, 135, 4, 90, 184, 230, 167, 249, 27, 69, 198, 152, 122, 36,
    248, 166, 68, 26, 153, 199, 37, 123, 58, 100, 134, 216, 91, 5, 231, 185,
    140, 210, 48, 110, 237, 179, 81, 15, 78, 16, 242, 172, 47, 113, 147, 205,
    17, 79, 173, 243, 112, 46, 204, 146, 211, 141, 111, 49, 178, 236, 14, 80,
    175, 241, 19, 77, 206, 144, 114, 44, 109, 51, 209, 143, 12, 82, 176, 238,
    50, 108, 142, 208, 83, 13, 239, 177, 240, 174, 76, 18, 145, 207, 45, 115,
    202, 148, 118, 40, 171, 245, 23, 73, 8, 86, 180, 234, 105, 55, 213, 139,
    87, 9, 235, 181, 54, 104, 138, 212, 149, 203, 41, 119, 244, 170, 72, 22,
    233, 183, 85, 11, 136, 214, 52, 106, 43, 117, 151, 201, 74, 20, 246, 168,
    116, 42, 200, 150, 21, 75, 169, 247, 182, 232, 10, 84, 215, 137, 107, 53];

pub struct CRC8(pub u8);

impl CRC8 {
    pub fn new() -> Self {
        CRC8(0)
    }
    pub fn calc(&mut self, value: u8) {
        self.0 = DSCRC_TABLE[(self.0 ^ value) as usize];
    }
}
