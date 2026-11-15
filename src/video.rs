//! MP4 files of the pictures that the viewer draws, written by the `ffmpeg` executable.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

/// The program that writes the videos, found on PATH.
pub const FFMPEG: &str = "ffmpeg";

/// A video that is being written. `ffmpeg` gets every picture as it is drawn; the file has
/// the name with `.part` added until `finish` gives it its own.
pub struct Recorder {
    child: Child,
    input: Option<ChildStdin>,
    file: PathBuf,
    part: PathBuf,
    width: u32,
    height: u32,
    frames: usize,
}

impl Recorder {
    /// Starts `program` for a video of `width` x `height` pixels with `rate` pictures a
    /// second: a number or a quotient like `100/5`.
    pub fn start(
        program: &Path,
        file: &Path,
        width: u32,
        height: u32,
        rate: &str,
        lossy: bool,
    ) -> Result<Self, String> {
        let part = partial(file);
        let mut child = Command::new(program)
            .args(arguments(width, height, rate, lossy, &part))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|err| {
                format!(
                    "cannot run {}: {err}; a video needs the ffmpeg executable on PATH",
                    program.display()
                )
            })?;
        let input = child.stdin.take();
        Ok(Self {
            child,
            input,
            file: file.to_owned(),
            part,
            width,
            height,
            frames: 0,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The pictures that the video has so far.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Adds a picture: RGBA bytes with the rows from the bottom up, as OpenGL reads them.
    pub fn frame(&mut self, rgba: &[u8]) -> Result<(), String> {
        if rgba.len() != self.width as usize * self.height as usize * 4 {
            return Err(format!(
                "a picture of {} bytes does not fit a video of {} x {} pixels",
                rgba.len(),
                self.width,
                self.height
            ));
        }
        let written = match &mut self.input {
            Some(input) => input.write_all(rgba),
            None => return Err("the video is closed".to_owned()),
        };
        if let Err(err) = written {
            // ffmpeg has said why on the standard error of this program.
            self.input = None;
            let status = self
                .child
                .wait()
                .map_or(String::new(), |status| format!(" ({status})"));
            let _ = std::fs::remove_file(&self.part);
            return Err(format!(
                "ffmpeg stopped before the video was complete{status}: {err}"
            ));
        }
        self.frames += 1;
        Ok(())
    }

    /// Ends the video: `ffmpeg` writes what is left and the file gets its name. Returns the
    /// number of its pictures.
    pub fn finish(mut self) -> Result<usize, String> {
        if self.frames == 0 {
            // Dropping it removes what there is.
            return Err("the video has no picture".to_owned());
        }
        drop(self.input.take());
        let status = self.child.wait();
        match status {
            Ok(status) if status.success() => {}
            failure => {
                let _ = std::fs::remove_file(&self.part);
                return Err(match failure {
                    Ok(status) => format!("ffmpeg failed ({status})"),
                    Err(err) => format!("cannot wait for ffmpeg: {err}"),
                });
            }
        }
        std::fs::rename(&self.part, &self.file).map_err(|err| {
            format!(
                "cannot rename {} to {}: {err}",
                self.part.display(),
                self.file.display()
            )
        })?;
        Ok(self.frames)
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        // Only a video that was not finished still has its pipe.
        if self.input.take().is_some() {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = std::fs::remove_file(&self.part);
        }
    }
}

/// Makes sure that the video can be written before the values are calculated: `program`
/// encodes one small picture next to `file`, with the settings of the video.
pub fn check(program: &Path, file: &Path, rate: &str, lossy: bool) -> Result<(), String> {
    let mut name = file.as_os_str().to_owned();
    name.push(".probe");
    let probe = PathBuf::from(name);
    let mut recorder = Recorder::start(program, &probe, PROBE, PROBE, rate, lossy)?;
    recorder.frame(&[0; (PROBE * PROBE * 4) as usize])?;
    recorder.finish()?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

/// The side of the picture of `check`: H.265 refuses very small ones.
const PROBE: u32 = 64;

fn partial(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

/// The arguments of `ffmpeg`: the pictures arrive on its standard input and `output` is an
/// MP4 file whatever its name is.
fn arguments(width: u32, height: u32, rate: &str, lossy: bool, output: &Path) -> Vec<OsString> {
    let size = format!("{width}x{height}");
    let input = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgba",
        "-video_size",
        &size,
        "-framerate",
        rate,
        "-i",
        "pipe:0",
        "-an",
    ];
    // The rows arrive from the bottom up. Without `--lossy` every frame is a PNG picture of
    // its own, compressed as far as the format goes: what is read back is what was drawn.
    // H.265 has even sides and the colours of BT.709, which players expect of it.
    let codec: &[&str] = if lossy {
        &[
            "-vf",
            "vflip,crop=trunc(iw/2)*2:trunc(ih/2)*2:0:0,\
             scale=out_color_matrix=bt709:out_range=tv,\
             setparams=color_primaries=bt709:color_trc=bt709",
            "-c:v",
            "libx265",
            "-preset",
            "medium",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-tag:v",
            "hvc1",
            "-x265-params",
            "log-level=error",
            "-movflags",
            "+faststart",
        ]
    } else {
        &[
            "-vf",
            "vflip",
            "-c:v",
            "png",
            "-pix_fmt",
            "rgb24",
            "-compression_level",
            "9",
            "-pred",
            "none",
        ]
    };
    let mut arguments: Vec<OsString> = input
        .iter()
        .chain(codec)
        .chain(&["-f", "mp4"])
        .map(OsString::from)
        .collect();
    // `file:` keeps a name with a colon, or one that starts with a dash, a file name.
    let mut target = OsString::from("file:");
    target.push(output);
    arguments.push(target);
    arguments
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    /// A program in `directory` that takes the place of ffmpeg: a shell script that has its
    /// output file, the last argument without `file:`, in `$out`.
    fn script(directory: &Path, body: &str) -> PathBuf {
        let program = directory.join("ffmpeg");
        fs::write(
            &program,
            format!("#!/bin/sh\nfor out; do :; done\nout=${{out#file:}}\n{body}\n"),
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
        program
    }

    #[test]
    fn a_video_gets_its_pictures_as_they_come_and_its_name_when_it_is_finished() {
        let directory = tempfile::tempdir().unwrap();
        let program = script(
            directory.path(),
            "echo \"$@\" > \"$out.args\"\ncat > \"$out\"",
        );
        let file = directory.path().join("line: 1.mp4");
        let part = directory.path().join("line: 1.mp4.part");
        let mut recorder = Recorder::start(&program, &file, 3, 2, "100/5", false).unwrap();
        assert_eq!(recorder.size(), (3, 2));
        recorder.frame(&[1; 24]).unwrap();
        recorder.frame(&[2; 24]).unwrap();
        assert!(recorder
            .frame(&[3; 23])
            .unwrap_err()
            .contains("does not fit a video of 3 x 2 pixels"));
        assert_eq!(recorder.frames(), 2);
        assert!(!file.exists());
        assert_eq!(recorder.finish(), Ok(2));
        assert!(!part.exists());
        let mut pictures = vec![1; 24];
        pictures.extend([2; 24]);
        assert_eq!(fs::read(&file).unwrap(), pictures);
        let arguments = fs::read_to_string(directory.path().join("line: 1.mp4.part.args")).unwrap();
        assert!(arguments.contains("-video_size 3x2 -framerate 100/5 -i pipe:0"));
        assert!(arguments.contains("-c:v png") && !arguments.contains("libx265"));
        let mut lossy = Recorder::start(&program, &file, 3, 2, "7/3", true).unwrap();
        lossy.frame(&[3; 24]).unwrap();
        assert_eq!(lossy.finish(), Ok(1));
        assert_eq!(fs::read(&file).unwrap(), [3; 24]);
        let arguments = fs::read_to_string(directory.path().join("line: 1.mp4.part.args")).unwrap();
        assert!(arguments.contains("-framerate 7/3 "));
        assert!(arguments.contains("-c:v libx265") && !arguments.contains("png"));
    }

    #[test]
    fn a_failure_of_ffmpeg_is_reported_and_leaves_no_file() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("a.mp4");
        let part = directory.path().join("a.mp4.part");
        let leftovers = || file.exists() || part.exists();

        // It fails at the end.
        let program = script(directory.path(), "cat > \"$out\"\nexit 3");
        let mut recorder = Recorder::start(&program, &file, 2, 2, "20", false).unwrap();
        recorder.frame(&[0; 16]).unwrap();
        assert_eq!(
            recorder.finish().unwrap_err(),
            "ffmpeg failed (exit status: 3)"
        );
        assert!(!leftovers());

        // It stops before it has read a picture, which is larger than a pipe holds.
        let program = script(directory.path(), "echo started > \"$out\"\nexit 1");
        let mut recorder = Recorder::start(&program, &file, 300, 300, "20", false).unwrap();
        let error = recorder.frame(&[0; 360_000]).unwrap_err();
        assert!(
            error.starts_with("ffmpeg stopped before the video was complete (exit status: 1)"),
            "{error}"
        );
        assert!(recorder.frame(&[0; 360_000]).is_err());
        assert!(!leftovers());

        // The video is given up, or ended without a picture.
        let program = script(directory.path(), "cat > \"$out\"");
        let mut recorder = Recorder::start(&program, &file, 2, 2, "20", false).unwrap();
        recorder.frame(&[0; 16]).unwrap();
        drop(recorder);
        let recorder = Recorder::start(&program, &file, 2, 2, "20", false).unwrap();
        assert_eq!(recorder.finish().unwrap_err(), "the video has no picture");
        assert!(!leftovers());

        let missing = directory.path().join("none").join("ffmpeg");
        let error = Recorder::start(&missing, &file, 2, 2, "20", false)
            .err()
            .unwrap();
        assert!(error.contains("a video needs the ffmpeg executable on PATH"));
    }

    #[test]
    fn the_check_encodes_a_picture_and_leaves_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("a.mp4");
        let program = script(
            directory.path(),
            "cat > \"$out\"\nwc -c < \"$out\" > \"$0.size\"",
        );
        check(&program, &file, "20", true).unwrap();
        let size = fs::read_to_string(directory.path().join("ffmpeg.size")).unwrap();
        assert_eq!(size.trim(), (PROBE * PROBE * 4).to_string());
        let program = script(directory.path(), "cat > /dev/null\nexit 1");
        assert!(check(&program, &file, "20", false).is_err());
        let nowhere = directory.path().join("none").join("a.mp4");
        let program = script(directory.path(), "{ cat > \"$out\"; } 2> /dev/null");
        assert!(check(&program, &nowhere, "20", false).is_err());
        let left: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name != "ffmpeg" && name != "ffmpeg.size")
            .collect();
        assert!(left.is_empty(), "{left:?}");
    }

    /// The real program, where there is one: what is read back from a lossless video is what
    /// went in, the right way up, and a lossy video has the same frames, nearly.
    #[test]
    fn ffmpeg_gives_the_pictures_back() {
        let program = Path::new(FFMPEG);
        let available = Command::new(program)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !available.is_ok_and(|status| status.success()) {
            eprintln!("skipped: there is no ffmpeg on PATH");
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let (width, height) = (65_usize, 33_usize);
        // Rows from the bottom up; the frames differ and so do the rows and the columns.
        let picture = |frame: usize| -> Vec<u8> {
            let mut rgba = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    rgba.extend([(x * 3) as u8, (y * 7) as u8, (frame * 40) as u8, 200]);
                }
            }
            rgba
        };
        let decode = |file: &Path| -> Vec<u8> {
            let output = Command::new(program)
                .args(["-v", "error", "-i"])
                .arg(file)
                .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
                .output()
                .unwrap();
            assert!(output.status.success());
            output.stdout
        };
        let record = |name: &str, lossy: bool| -> PathBuf {
            let file = directory.path().join(name);
            check(program, &file, "5/2", lossy).unwrap();
            let mut recorder =
                Recorder::start(program, &file, width as u32, height as u32, "5/2", lossy).unwrap();
            for frame in 0..5 {
                recorder.frame(&picture(frame)).unwrap();
            }
            assert_eq!(recorder.finish(), Ok(5));
            file
        };

        // Top down, without the alpha, as a player shows them.
        let mut expected = Vec::new();
        for frame in 0..5 {
            let rgba = picture(frame);
            for y in (0..height).rev() {
                for pixel in rgba[y * width * 4..(y + 1) * width * 4].chunks(4) {
                    expected.extend(&pixel[..3]);
                }
            }
        }
        assert_eq!(decode(&record("-a: b.mp4", false)), expected);

        // H.265 has even sides: the last column and the last row are cut.
        let decoded = decode(&record("lossy.mp4", true));
        assert_eq!(decoded.len(), 5 * 64 * 32 * 3);
        let mut error = 0;
        for (index, pixel) in decoded.chunks(3).enumerate() {
            let (frame, y, x) = (index / (64 * 32), index / 64 % 32, index % 64);
            let at = (frame * height + y) * width + x;
            for channel in 0..3 {
                error += i64::from(pixel[channel]).abs_diff(i64::from(expected[at * 3 + channel]));
            }
        }
        assert!(error / (decoded.len() as u64) < 12, "{error}");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
