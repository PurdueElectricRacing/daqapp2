pub const CAN_EFF_FLAG: u32 = 0x80000000;
pub const CAN_EXT_ID_MASK: u32 = 0x1FFFFFFF;
pub const CAN_STD_ID_MASK: u32 = 0x000007FF;

pub const FRAME_TYPE_OFFSET: usize = 1;
pub const MSG_BYTE_LEN: usize = 19;
pub const TIMESTAMP_OFFSET: usize = 5;
pub const ID_OFFSET: usize = 9;
pub const DLC_OFFSET: usize = 10;
pub const DATA_OFFSET: usize = 11;

pub const MAX_JUMP_MS: u32 = 300_000; // 300 seconds

pub const BIN_WIDTH_MS: u32 = 15;
