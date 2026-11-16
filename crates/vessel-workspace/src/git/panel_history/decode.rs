//! Strict NUL-field history decoding. Git's -z terminator yields six fields
//! per record; record-separator bytes in a subject are ordinary content.
//! This does not change the old bounded log API's legacy decoder.
use super::{HistoryError, HistoryField};
use chrono::DateTime;
use vessel_proto::workspace::{CommitAbbreviation, CommitId, CommitInfo};

const FIELDS: usize = 6;

pub(super) fn records(output: &[u8]) -> Result<Vec<Metadata>, HistoryError> {
    if output.is_empty() {
        return Ok(Vec::new());
    }
    if !output.ends_with(&[0]) {
        return Err(HistoryError::Record(HistoryField::Terminator));
    }
    let text = std::str::from_utf8(output)?;
    let mut fields = text.split_terminator('\0');
    let mut commits = Vec::new();
    while let Some(id) = fields.next() {
        let parents = fields
            .next()
            .ok_or(HistoryError::Record(HistoryField::Parents))?;
        let short = fields
            .next()
            .ok_or(HistoryError::Record(HistoryField::Abbreviation))?;
        let author = fields
            .next()
            .ok_or(HistoryError::Record(HistoryField::Author))?;
        let time = fields
            .next()
            .ok_or(HistoryError::Record(HistoryField::Time))?;
        let subject = fields
            .next()
            .ok_or(HistoryError::Record(HistoryField::Subject))?;
        commits.push(commit([id, parents, short, author, time, subject])?);
    }
    Ok(commits)
}

pub(super) struct Metadata {
    pub(super) info: CommitInfo,
    pub(super) short: CommitAbbreviation,
}

fn commit(fields: [&str; FIELDS]) -> Result<Metadata, HistoryError> {
    let [id, parents, short, author, time, subject] = fields;
    let id: CommitId = id.parse()?;
    let short: CommitAbbreviation = short.parse()?;
    if !matches!(id.as_str().len(), 40 | 64) {
        return Err(HistoryError::Record(HistoryField::Object));
    }
    if !id.as_str().starts_with(short.as_str()) {
        return Err(HistoryError::Record(HistoryField::Abbreviation));
    }
    let seconds = time.parse::<i64>()?;
    let time =
        DateTime::from_timestamp(seconds, 0).ok_or(HistoryError::Record(HistoryField::Time))?;
    Ok(Metadata {
        short,
        info: CommitInfo {
            id,
            parents: if parents.is_empty() {
                Vec::new()
            } else {
                parents
                    .split(' ')
                    .map(str::parse)
                    .collect::<Result<_, _>>()?
            },
            author: author.to_owned(),
            time,
            subject: subject.to_owned(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(subject: &str) -> Vec<u8> {
        format!("{}\0\0aaaaaaaa\0Author\0-1\0{subject}\0", "a".repeat(40)).into_bytes()
    }

    #[test]
    fn separators_and_long_subjects_survive_without_an_epoch_or_parent_guess() {
        let subject = format!("field\u{1e}{}", "x".repeat(3000));
        let parsed = records(&record(&subject)).unwrap();
        assert_eq!(parsed[0].info.subject, subject);
        assert!(parsed[0].info.parents.is_empty());
        assert_eq!(parsed[0].info.time.timestamp(), -1);
        assert_eq!(parsed[0].info.author, "Author");
    }
    #[test]
    fn malformed_metadata_fails_the_whole_read_instead_of_dropping_a_record() {
        for bad in [
            record("ok")[..10].to_vec(),
            b"bad\0\0aaaaaaaa\0Author\x001\0subject\0".to_vec(),
            format!(
                "{}\0invalid-parent\0aaaaaaaa\0Author\x001\0subject\0",
                "a".repeat(40)
            )
            .into_bytes(),
            format!(
                "{}\0\0aaaaaaaa\0Author\0bad-time\0subject\0",
                "a".repeat(40)
            )
            .into_bytes(),
        ] {
            assert!(records(&bad).is_err());
        }
        let mut invalid = record("subject");
        invalid[0] = 255;
        assert!(matches!(records(&invalid), Err(HistoryError::Utf8(_))));
    }
    #[test]
    fn empty_subjects_and_consecutive_records_keep_their_exact_metadata() {
        let mut output = record("");
        output.extend(record("next"));
        let parsed = records(&output).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].info.subject, "");
        assert_eq!(parsed[1].info.subject, "next");
    }
}
