// identity bit masks
pub const IS_EID_MASK: u32 = 0x40000000;
pub const MAX_JUMP_MS: u32 = 300_000; // 300 seconds
pub const BUS_ID_MASK: u32 = 0x80000000; // BUS ID is in the highest bit of the identity

pub const BIN_WIDTH_MS: u32 = 1;

pub const NO_CONNECTION_SLEEP_MS: u64 = 200;
pub const READ_RETRY_SLEEP_MS: u64 = 2;
pub const BUS_LOAD_UPDATE_MS: u128 = 200;
pub const LOG_FRAMES_MS: u128 = 60000;

pub const LOG_FOLDER_PATH: &str = "logs";
