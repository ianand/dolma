use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use threadpool::ThreadPool;

use crate::shard::Shard;

use mixer_config::*;

pub fn run(config: MixerConfig) {
    let shards = Shard::split_streams(&config.streams).unwrap();

    let threadpool = ThreadPool::new(config.processes);
    let failed_shard_count = AtomicU32::new(0);
    let failed_shard_count_ref = Arc::new(failed_shard_count);
    for shard in shards {
        let output_path = Path::new(&config.work_dir.output.clone()).join(&shard.output);
        if output_path.exists() {
            log::info!("Skipping {:?} because it already exists", shard.output);
            continue;
        }
        let shard = shard.clone();
        let work_dirs = config.work_dir.clone();
        let failed_shard_count_ref = failed_shard_count_ref.clone();

        threadpool.execute(move || {
            log::info!("Building output {:?}...", shard.output);
            match shard.clone().process(work_dirs) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error processing {:?}: {}", shard.output, e);
                    failed_shard_count_ref.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    threadpool.join();

    let failure_count = failed_shard_count_ref.fetch_add(0, Ordering::Relaxed);
    if failure_count > 0 {
        log::error!("{} shards failed to process.", failure_count);
        process::exit(1);
    } else {
        log::info!("Done!");
    }
}

pub mod mixer_config {
    use serde::{Deserialize, Serialize};
    use std::fs::File;
    use std::io;

    use crate::shard::shard_config::{StreamConfig, WorkDirConfig};

    #[derive(Serialize, Deserialize)]
    pub struct MixerConfig {
        pub streams: Vec<StreamConfig>,
        pub processes: usize,
        pub work_dir: WorkDirConfig,
    }

    impl MixerConfig {
        pub fn read_from_file(path: &str) -> Result<MixerConfig, io::Error> {
            let file = File::open(path)?;
            let reader = io::BufReader::new(file);
            let config: MixerConfig = serde_json::from_reader(reader)?;
            Ok(config)
        }
        pub fn parse_from_string(s: &str) -> Result<MixerConfig, io::Error> {
            let config: MixerConfig = serde_json::from_str(s)?;
            Ok(config)
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs::OpenOptions;
    use std::io;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    use flate2::read::MultiGzDecoder;

    use crate::s3_util;
    use crate::s3_util::download_to_file;

    use super::*;

    fn compare_contents(expected: &str, actual: &str) {
        let expected_lines = BufReader::new(MultiGzDecoder::new(
            OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(expected)
                .unwrap(),
        ))
        .lines()
        .collect::<Vec<Result<String, io::Error>>>();
        let actual_lines = BufReader::new(MultiGzDecoder::new(
            OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(actual)
                .unwrap(),
        ))
        .lines()
        .collect::<Vec<Result<String, io::Error>>>();

        assert_eq!(
            expected_lines.len(),
            actual_lines.len(),
            "Wrong number of output documents"
        );

        for (actual, expected) in std::iter::zip(expected_lines, actual_lines) {
            let actual = actual.unwrap();
            let expected = expected.unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_mixer() -> Result<(), io::Error> {
        let config = MixerConfig::read_from_file("tests/config/mixer.json")?;
        run(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let s3_client = s3_util::new_client(None)?;

        let local_output_file = "tests/work/output/mixer.json.gz";
        let remote_output_file =
            "s3://ai2-llm/pretraining-data/tests/mixer/outputs/v1/documents/head/mixer-test-0000.json.gz";
        rt.block_on(download_to_file(
            &s3_client,
            remote_output_file,
            Path::new(local_output_file),
        ))?;

        compare_contents("tests/data/expected/mixer.json.gz", local_output_file);
        Ok(())
    }

    #[test]
    fn test_email_span_replacement() -> Result<(), io::Error> {
        let config = MixerConfig::read_from_file("tests/config/email-spans.json")?;
        run(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let s3_client = s3_util::new_client(None)?;

        let local_output_file = "tests/work/output/email-spans.json.gz";
        let remote_output_file =
            "s3://ai2-llm/pretraining-data/tests/mixer/outputs/v1/documents/head/email-spans-test-0000.json.gz";
        rt.block_on(download_to_file(
            &s3_client,
            remote_output_file,
            Path::new(local_output_file),
        ))?;

        compare_contents("tests/data/expected/email-spans.json.gz", local_output_file);
        Ok(())
    }

    #[test]
    fn test_paragraph_removal() -> Result<(), io::Error> {
        let config = MixerConfig::read_from_file("tests/config/paragraph-spans.json")?;
        run(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let s3_client = s3_util::new_client(None)?;

        let local_output_file = "tests/work/output/remove-paragraphs.json.gz";
        let remote_output_file =
            "s3://ai2-llm/pretraining-data/tests/mixer/outputs/v1/documents/head/paragraph-spans-test-0000.json.gz";
        rt.block_on(download_to_file(
            &s3_client,
            remote_output_file,
            Path::new(local_output_file),
        ))?;

        compare_contents(
            "tests/data/expected/remove-paragraphs.json.gz",
            local_output_file,
        );
        Ok(())
    }

    #[test]
    fn test_filter_by_span() -> Result<(), io::Error> {
        let config = MixerConfig::read_from_file("tests/config/filter-by-spans.json")?;
        run(config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let s3_client = s3_util::new_client(None)?;

        let local_output_file = "tests/work/output/filter-by-spans.json.gz";
        let remote_output_file =
            "s3://ai2-llm/pretraining-data/tests/mixer/outputs/v1/documents/head/filter-by-spans-test-0000.json.gz";
        rt.block_on(download_to_file(
            &s3_client,
            remote_output_file,
            Path::new(local_output_file),
        ))?;

        compare_contents(
            "tests/data/expected/filter-by-spans.json.gz",
            local_output_file,
        );
        Ok(())
    }
}
