use image::{GrayImage, ImageReader, ImageBuffer, Luma};
use std::fs;

// Stores the computed average grayscale image
pub struct ImageAveragerFromBuffer
{
    average_img: GrayImage,
}

#[allow(dead_code)]
impl ImageAveragerFromBuffer
{
    // Create an empty averager (0x0 image)
    pub fn new() -> ImageAveragerFromBuffer
    {
        return ImageAveragerFromBuffer
        {
            average_img: ImageBuffer::new(0, 0)
        }
    }

    // Create averager and compute average from input images
    pub fn new_with_source(source: Vec<ImageBuffer<Luma<u8>, Vec<u8>>>) -> ImageAveragerFromBuffer
    {
        return ImageAveragerFromBuffer
        {
            average_img: Self::find_average(source)
        }
    }

    // Compute per-pixel average across all images
    pub fn find_average(source: Vec<ImageBuffer<Luma<u8>, Vec<u8>>>) -> GrayImage
    {
        // Accumulator uses u32 to prevent overflow during summation
        let mut avg: ImageBuffer<Luma<u32>, Vec<u32>> =
            ImageBuffer::new(source[0].width(), source[0].height());

        // Final output image (u8 grayscale)
        let mut output: GrayImage =
            ImageBuffer::new(avg.width(), avg.height());

        // Number of input images
        let sample_size = source.iter().count() as u32;

        // Sum pixel values across all images
        for buf in source
        {
            for x in 0..buf.width()
            {
                for y in 0..buf.height()
                {
                    // Safe accumulation (no early clipping like u8)
                    avg[(x, y)][0] =
                        avg[(x, y)][0].saturating_add(buf[(x, y)][0] as u32);
                }
            }
        }

        // Normalize sum to get average
        for x in 0..avg.width()
        {
            for y in 0..avg.height()
            {
                if sample_size > 0
                {
                    // Divide once, then cast back to u8
                    output[(x, y)][0] =
                        (avg[(x, y)][0] / sample_size) as u8;
                }
                else
                {
                    // Fallback for empty input (should not normally occur)
                    output[(x, y)][0] = 0;
                }
            }
        }

        return output;
    }

    // Return a copy of the average image
    #[allow(dead_code)]
    pub fn get_average(&self) -> GrayImage
    {
        return self.average_img.clone();
    }

    // Subtract average image from input image (in-place)
    #[allow(dead_code)]
    pub fn apply_average(&self, img: &mut ImageBuffer<Luma<u8>, Vec<u8>>)
    {
        for x in 0..img.width()
        {
            for y in 0..img.height()
            {
                // Compute pixel difference using wider type to avoid underflow
                let mut brightness_value =
                    (img[(x, y)][0] as i16) - (self.average_img[(x, y)][0] as i16);

                // Clamp negatives to 0 (valid u8 range)
                if brightness_value < 0
                {
                    brightness_value = 0;
                }

                // Write adjusted pixel back
                img[(x, y)][0] = brightness_value as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;
    use image::{ImageBuffer, ImageReader, Luma};
    use crate::img_avg::ImageAveragerFromBuffer;

    #[test]
    fn test_averaging_by_folder()
    {
        let source_dir = fs::read_dir("./machines/pi-5/odin-compute/src/unit_test/source_images/").unwrap();
        let apply_dir = fs::read_dir("./machines/pi-5/odin-compute/src/unit_test/images_to_apply/").unwrap();

        let mut source_buf_vec: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = vec![];
        let mut apply_buf_vec: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = vec![];

        for path in source_dir
        {
            let img = ImageReader::open(path.unwrap().path().display().to_string()).unwrap();
            let img = img.with_guessed_format().unwrap();
            let img = img.decode().unwrap();
            let img = img.as_luma8().unwrap();

            source_buf_vec.push(img.clone());
        }

        for path in apply_dir
        {
            let img = ImageReader::open(path.unwrap().path().display().to_string()).unwrap();
            let img = img.with_guessed_format().unwrap();
            let img = img.decode().unwrap();
            let img = img.as_luma8().unwrap();

            apply_buf_vec.push(img.clone());
        }

        let avger = ImageAveragerFromBuffer::new_with_source(source_buf_vec);

        let mut i = 0;
        for ref mut buf in apply_buf_vec
        {
            let output_path = String::from("./machines/pi-5/odin-compute/src/unit_test/output_images/") + i.to_string().as_str() + ".tiff";

            avger.apply_average(buf);
            buf.clone().save(output_path).unwrap();

            i = i + 1;
        }

    }
}