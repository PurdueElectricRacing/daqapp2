// pub const CAN_EFF_FLAG: u32 = 0x80000000;
pub const CAN_EID_MASK: u32 = 0x1FFFFFFF;
pub const CAN_STD_ID_MASK: u32 = 0x000007FF;

// identity bit masks
// pub const BUS_ID_MASK: u32 = 0x80000000; // bit 31
pub const IS_EID_MASK: u32 = 0x40000000;
pub const MAX_JUMP_MS: u32 = 300_000; // 300 seconds

pub const BIN_WIDTH_MS: u32 = 1;
