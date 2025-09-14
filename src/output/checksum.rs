use crate::layout::settings::CrcData;
use crc::{Algorithm, Crc};
use std::sync::OnceLock;

static CRC_ALGORITHM: OnceLock<Algorithm<u32>> = OnceLock::new();

pub fn init_crc_algorithm(crc_settings: &CrcData) {
    let algo = Algorithm::<u32> {
        width: 32,
        poly: crc_settings.polynomial,
        init: crc_settings.start,
        refin: crc_settings.ref_in,
        refout: crc_settings.ref_out,
        xorout: crc_settings.xor_out,
        check: 0,
        residue: 0,
    };

    CRC_ALGORITHM.set(algo).ok(); // Don't panic if already initialized (useful for tests)
}

pub fn calculate_crc(data: &[u8]) -> u32 {
    let algorithm = CRC_ALGORITHM.get().expect("CRC algorithm not initialized");
    let crc = Crc::<u32>::new(algorithm);
    let mut crc_digest = crc.digest();
    crc_digest.update(data);
    crc_digest.finalize()
}
