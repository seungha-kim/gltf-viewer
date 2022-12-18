pub fn white_image() -> image::DynamicImage {
    let buffer = image::ImageBuffer::from_fn(1, 1, |_x, _y| {
        image::Luma([255u8])
    });
    buffer.into()
}
