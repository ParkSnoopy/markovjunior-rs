use crate::{
    error::{ModelError, error},
    grid::Grid,
};

impl Grid {
    /// Encodes one model in `MagicaVoxel` VOX v150 format using the default palette.
    /// The first declared grid value is empty space.
    ///
    /// # Errors
    /// Returns an error if any axis exceeds 256 cells.
    pub fn to_vox(&self) -> Result<Vec<u8>, ModelError> {
        let dimensions = [self.width, self.height, self.depth];
        if dimensions.iter().any(|&side| side > 256) {
            return Err(error("VOX supports at most 256 cells per axis"));
        }
        let count = self.cells.iter().filter(|&&value| value != 0).count() as u32;
        let data_len = 4 + 4 * count;
        let mut bytes = Vec::new();
        bytes.extend(b"VOX ");
        bytes.extend(150_u32.to_le_bytes());
        header(&mut bytes, *b"MAIN", 0, 24 + 12 + data_len);
        header(&mut bytes, *b"SIZE", 12, 0);
        for size in dimensions {
            bytes.extend((size as u32).to_le_bytes());
        }
        header(&mut bytes, *b"XYZI", data_len, 0);
        bytes.extend(count.to_le_bytes());
        for (i, &value) in self.cells.iter().enumerate() {
            if value != 0 {
                let row = i / self.width;
                bytes.extend([
                    (i % self.width) as u8,
                    (row % self.height) as u8,
                    (row / self.height) as u8,
                    value,
                ]);
            }
        }
        Ok(bytes)
    }
}

fn header(bytes: &mut Vec<u8>, id: [u8; 4], size: u32, children: u32) {
    bytes.extend(id);
    bytes.extend(size.to_le_bytes());
    bytes.extend(children.to_le_bytes());
}
