use crate::{approx, draw};

use std::path::PathBuf;

use ffmpeg_next::{codec, format, Rational, Packet, Error};
use ffmpeg_next::frame::Video;
use ffmpeg_next::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next::util::format::pixel::Pixel;
use image::{DynamicImage, ImageBuffer};

pub fn run(source: &PathBuf, output: &PathBuf, board_width: usize, board_height: usize) {
    const NUM_THREADS: usize = 8;

    ffmpeg_next::init().expect("failed to initialize ffmpeg");
    let (meta_data, input) = VideoInput::new(source);
    let mut output = VideoOutput::new(output, &meta_data);
    let config = draw::Config {
        board_width: board_width,
        board_height: board_height,
    };

    for (i, frame) in input.enumerate() {
        let approx_img = approx::approximate(&mut frame_to_image(frame, &meta_data), &config).unwrap();
        output.send_frame(&approx_img);

        if (i + 1) % 10 == 0 {
            println!("Approximated {} frames", i + 1);
        }
    }
}

type RawFrame = Vec<u8>;

// contains important video metadata
#[derive(Debug, Clone, Copy)]
struct VideoMetaData {
    width: u32,
    height: u32,
    fps: Rational,
    time_base: Rational,
    format: Pixel,
}

// streams from the input file
struct VideoInput {
    source: ffmpeg_next::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg_next::codec::decoder::video::Video,
    scaler: Context,
}

// streams to the output file
struct VideoOutput {
    output: ffmpeg_next::format::context::Output,
    video_stream_index: usize,
    encoder: ffmpeg_next::codec::encoder::Video,
    scaler: Context,
}

impl VideoInput {
    fn new(file_name: &PathBuf) -> (VideoMetaData, VideoInput) {
        let source = format::input(file_name).unwrap();
        let input = source.streams().best(ffmpeg_next::media::Type::Video).unwrap();

        // video metadata
        let video_stream_index = input.index();
        let fps = input.avg_frame_rate();
        let time_base = input.time_base();
        let format = Pixel::RGB24;

        // setup the decoder
        let decoder = input.codec().decoder().video().unwrap();
        let scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            format,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        ).unwrap();

        (VideoMetaData {
            width: decoder.width(),
            height: decoder.height(),
            fps: fps,
            time_base: time_base,
            format: format,
        },
        VideoInput {
            source: source,
            video_stream_index: video_stream_index,
            decoder: decoder,
            scaler: scaler,
        })
    }
}

impl Iterator for VideoInput {
    type Item = RawFrame;
    fn next(&mut self) -> Option<Self::Item> {
        // Loop through the packets in the video file
        while let Some((stream, packet)) = self.source.packets().next() {
            if stream.index() == self.video_stream_index {
                // send the video packet to the codec
                self.decoder.send_packet(&packet).unwrap();
                
                // then attempt to decode and collect that packet into a frame
                let mut frame = Video::empty();
                if self.decoder.receive_frame(&mut frame).is_ok() {
                    // then scale the frame, which is now ready to push
                    let mut rgb_frame = Video::empty();
                    self.scaler.run(&frame, &mut rgb_frame).unwrap();
                    let data = rgb_frame.data(0).to_vec();
                    return Some(data)
                }
            }
        }

        // Flush the decoder to get frames from remaining packets
        let mut frame = Video::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            let mut rgb_frame = Video::empty();
            self.scaler.run(&frame, &mut rgb_frame).unwrap();

            // Collect the frame data
            let data = rgb_frame.data(0).to_vec();
            return Some(data)
        }

        None
    }
}

impl VideoOutput {
    fn new(file_name: &PathBuf, video: &VideoMetaData) -> VideoOutput {
        // open the output file
        let mut output = format::output(file_name).expect("failed to open output file");

        // add a video stream
        let codec = codec::encoder::find(codec::Id::H264).expect("failed to find H264 codec");
        let stream = output.add_stream(codec).expect("failed to add stream to output");

        let mut encoder = stream.codec().encoder().video().expect("failed to create encoder");
        encoder.set_width(video.width);
        encoder.set_height(video.height);
        encoder.set_format(Pixel::YUV420P);
        encoder.set_time_base(video.time_base);
        encoder.set_frame_rate(Some(video.fps));

        // Open the encoder to ensure all parameters are set correctly
        let encoder = encoder.open_as(codec).expect("failed to open encoder");

        // Get the video stream index
        let video_stream_index = stream.index();

        let scaler = Context::get(
            video.format,
            encoder.width(),
            encoder.height(),
            video.format,
            encoder.width(),
            encoder.height(),
            Flags::BILINEAR,
        )
        .expect("failed to create scaler");
        VideoOutput {
            output: output,
            video_stream_index: video_stream_index,
            encoder: encoder,
            scaler: scaler,
        }
    }

    fn send_frame(&mut self, image: &DynamicImage) {
        let mut rgb_frame = Video::empty();
        let data = image.to_rgb8().into_vec();
        rgb_frame.set_format(Pixel::RGB24);
        rgb_frame.set_width(image.width());
        rgb_frame.set_height(image.height());
        rgb_frame.data_mut(0).copy_from_slice(&data);
        self.encoder.send_frame(&rgb_frame).unwrap();
    }
}

fn frame_to_image(frame: RawFrame, video: &VideoMetaData) -> DynamicImage {
    let buffer: ImageBuffer<image::Rgb<u8>, _> = image::ImageBuffer::from_raw(video.width, video.height, frame.to_owned()).expect("failed to create image from raw data");
    DynamicImage::from(buffer)
}
