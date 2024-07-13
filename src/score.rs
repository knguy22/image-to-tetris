use imageproc::image::DynamicImage;
use image_compare::{self, Similarity, CompareError, rgb_hybrid_compare};

pub fn compare_images(image1: &image::DynamicImage, image2: &image::DynamicImage) -> Result<Similarity, CompareError> {
    rgb_hybrid_compare(&image1.clone().to_rgb8(), &image2.clone().into_rgb8())
}