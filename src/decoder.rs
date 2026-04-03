use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioData {
    pub samples: Vec<Vec<f32>>,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_secs: f64,
    pub codec: String,
    pub format_name: String,
    #[allow(dead_code)]
    pub total_samples: usize,
}

pub fn decode_file(path: &Path) -> Result<AudioData, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("no audio track found")?;

    let codec_params = track.codec_params.clone();
    let track_id = track.id;

    let sample_rate = codec_params.sample_rate.ok_or("unknown sample rate")?;
    let channels = codec_params
        .channels
        .map(|c| c.count())
        .ok_or("unknown channel count")?;

    let codec_name = symphonia::default::get_codecs()
        .get_codec(codec_params.codec)
        .map(|d| d.short_name.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let format_name = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    let mut decoder =
        symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default())?;

    let mut channel_samples: Vec<Vec<f32>> = vec![Vec::new(); channels];

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };

        let spec = *decoded.spec();
        let num_frames = decoded.frames();

        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let interleaved = sample_buf.samples();
        let ch = spec.channels.count();

        for (i, sample) in interleaved.iter().enumerate() {
            channel_samples[i % ch].push(*sample);
        }
    }

    let total_samples = channel_samples.first().map(|c| c.len()).unwrap_or(0);
    let duration_secs = total_samples as f64 / sample_rate as f64;

    Ok(AudioData {
        samples: channel_samples,
        sample_rate,
        channels,
        duration_secs,
        codec: codec_name,
        format_name,
        total_samples,
    })
}
