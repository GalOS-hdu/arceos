//! Block device adapters for lwext4_core
//!
//! 提供 AxBlockDevice 到 lwext4_core::BlockDevice 的适配器

use axdriver::AxBlockDevice;

/// Adapter for AxBlockDevice to work with lwext4_core
pub struct Ext4CoreDisk {
    inner: AxBlockDevice,
}

impl Ext4CoreDisk {
    /// Create a new adapter from AxBlockDevice
    pub fn new(dev: AxBlockDevice) -> Self {
        Self { inner: dev }
    }
}

impl lwext4_core::BlockDevice for Ext4CoreDisk {
    fn block_size(&self) -> u32 {
        use axdriver::prelude::BlockDriverOps;
        self.inner.block_size() as u32
    }

    fn sector_size(&self) -> u32 {
        // ext4 标准扇区大小为 512 字节
        512
    }

    fn total_blocks(&self) -> u64 {
        use axdriver::prelude::BlockDriverOps;
        self.inner.num_blocks()
    }

    fn read_blocks(&mut self, lba: u64, count: u32, buf: &mut [u8]) -> lwext4_core::Result<usize> {
        use axdriver::prelude::BlockDriverOps;

        let block_size = self.inner.block_size();
        let expected_size = count as usize * block_size;

        if buf.len() < expected_size {
            return Err(lwext4_core::Error::new(
                lwext4_core::ErrorKind::InvalidInput,
                "Buffer too small"
            ));
        }

        self.inner
            .read_block(lba, &mut buf[..expected_size])
            .map_err(|_| lwext4_core::Error::new(lwext4_core::ErrorKind::Io, "Block read failed"))?;

        Ok(expected_size)
    }

    fn write_blocks(&mut self, lba: u64, count: u32, buf: &[u8]) -> lwext4_core::Result<usize> {
        use axdriver::prelude::BlockDriverOps;

        let block_size = self.inner.block_size();
        let expected_size = count as usize * block_size;

        if buf.len() < expected_size {
            return Err(lwext4_core::Error::new(
                lwext4_core::ErrorKind::InvalidInput,
                "Buffer too small"
            ));
        }

        self.inner
            .write_block(lba, &buf[..expected_size])
            .map_err(|_| lwext4_core::Error::new(lwext4_core::ErrorKind::Io, "Block write failed"))?;

        Ok(expected_size)
    }
}
