use ffmpeg_next::{codec, format, Rational, Packet, Error};
use ffmpeg_next::frame::Video;
use ffmpeg_next::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next::util::format::pixel::Pixel;

pub struct VideoData {
    pub width: u32,
    pub height: u32,
    pub fps: i32,
    pub format: Pixel,
    pub frames: Vec<Vec<u8>>,
}

pub fn extract_rgb_frames(file_name: &str) -> VideoData {
    let mut source = format::input(file_name).unwrap();
    let input = source.streams().best(ffmpeg_next::media::Type::Video).unwrap();

    // video metadata
    let video_stream_index = input.index();
    let fps = input.avg_frame_rate();
    let format = Pixel::RGB24;

    // setup the decoder
    let mut codec_context = input.codec().decoder().video().unwrap();
    let mut scaler = Context::get(
        codec_context.format(),
        codec_context.width(),
        codec_context.height(),
        format,
        codec_context.width(),
        codec_context.height(),
        Flags::BILINEAR,
    ).unwrap();

    // Loop through the packets in the video file
    let mut decoded_frames = Vec::new();
    for (stream, packet) in source.packets() {
        // send the video packet to the codec
        if stream.index() == video_stream_index {
            codec_context.send_packet(&packet).unwrap();
            
            // then decode and collect that packet into a frame
            let mut frame = Video::empty();
            while codec_context.receive_frame(&mut frame).is_ok() {

                // then scale the frame, which is now ready to push
                let mut rgb_frame = Video::empty();
                scaler.run(&frame, &mut rgb_frame).unwrap();
                let data = rgb_frame.data(0).to_vec();
                decoded_frames.push(data);

                // clear the frame for the next iter
                frame = Video::empty();
            }
        }
    }
    println!("{} frames decoded", decoded_frames.len());

    // Flush the decoder to get the remaining frames
    codec_context.send_eof().unwrap();
    let mut frame = Video::empty();
    while codec_context.receive_frame(&mut frame).is_ok() {
        let mut rgb_frame = Video::empty();
        scaler.run(&frame, &mut rgb_frame).unwrap();

        // Collect the frame data
        let data = rgb_frame.data(0).to_vec();
        decoded_frames.push(data);

        // Clear the frame for the next iteration
        frame = Video::empty();
    }

    println!("{} frames decoded", decoded_frames.len());
    
    VideoData {
        width: codec_context.width(),
        height: codec_context.height(),
        fps: u32::from(fps) as i32,
        format: format,
        frames: decoded_frames,
    }
}

pub fn save_from_rgb_frames(video: &VideoData, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = format::output(&file_name)?;
    let global_header = output.format().flags().contains(format::flag::Flags::GLOBAL_HEADER);
    let mut stream = output.add_stream(codec::Id::H264)?;

    {
        let mut encoder = stream.codec().encoder().video()?;
        encoder.set_width(video.width);
        encoder.set_height(video.height);
        encoder.set_format(video.format);
        encoder.set_frame_rate(Some(Rational::new(video.fps, 1)));
        encoder.set_time_base(Rational::new(1, video.fps));
        if global_header {
            encoder.set_flags(codec::flag::Flags::GLOBAL_HEADER);
        }
    }

    // open output file
    output.write_header()?;

    // Create a scaler context to convert frames to YUV format
    let mut scaler = Context::get(
        video.format,
        video.width,
        video.height,
        Pixel::YUV420P,
        video.width,
        video.height,
        Flags::BILINEAR,
    )?;

    let mut pts = 0;
    let mut video_encoder = stream.codec().encoder().video()?;
    for rgb_data in video.frames.iter() {
        let mut rgb_frame = Video::empty();
        rgb_frame.set_format(video.format);
        rgb_frame.set_width(video.width);
        rgb_frame.set_height(video.height);
        rgb_frame.data_mut(0).copy_from_slice(&rgb_data);

        let mut yuv_frame = Video::empty();
        yuv_frame.set_format(Pixel::YUV410P);
        yuv_frame.set_width(video.width);
        yuv_frame.set_height(video.height);

        scaler.run(&rgb_frame, &mut yuv_frame)?;
        yuv_frame.set_pts(Some(pts));
        pts += 1;

        video_encoder.send_frame(&yuv_frame)?;

        // flush encoder
        video_encoder.send_eof()?;
        let mut encoded = Packet::empty();
        while video_encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(0);
            output.write_header(&encoded)?;
        }
    }


    Ok(())
}