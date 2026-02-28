use anyhow::{Context, Result};

pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).context("zstd compress failed")
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data).context("zstd decompress failed")
}

pub fn compress_with_dict(data: &[u8], level: i32, dict: &[u8]) -> Result<Vec<u8>> {
    let mut encoder =
        zstd::stream::Encoder::with_dictionary(Vec::new(), level, dict).context("encoder with dict")?;
    std::io::Write::write_all(&mut encoder, data).context("write payload to zstd encoder")?;
    encoder.finish().context("finalize zstd dict compression")
}

pub fn decompress_with_dict(data: &[u8], dict: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = zstd::stream::Decoder::with_dictionary(data, dict).context("decoder with dict")?;
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut out).context("read zstd dict payload")?;
    Ok(out)
}

pub fn train_dictionary(samples: &[Vec<u8>], dict_size: usize) -> Result<Vec<u8>> {
    if samples.is_empty() {
        return Ok(Vec::new());
    }
    zstd::dict::from_samples(samples, dict_size).context("dictionary training failed")
}
