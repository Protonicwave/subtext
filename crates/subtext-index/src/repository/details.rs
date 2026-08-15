//! What each film's file turned out to be.
//!
//! Read once, when the film is opened during a scan, and held on the row from
//! then on. Everything that shows a fact about a file reads it from here, so
//! opening a film's page costs no disk at all.

use std::collections::HashMap;

use rusqlite::{Connection, params};
use subtext_core::Timestamp;

use crate::database::Database;
use crate::error::Result;
use crate::model::{AudioDetails, MediaDetails};
use crate::repository::{from_sql_int, to_sql_int};

#[derive(Debug)]
pub struct Details<'a> {
    database: &'a Database,
}

impl<'a> Details<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Records what a batch of films turned out to be, in one transaction.
    ///
    /// Written whether or not anything changed, and unguarded, because a film
    /// only reaches this after being opened: a rescan of a library nobody has
    /// touched opens nothing, so there is no unchanged case to save.
    ///
    /// The running time is filled in rather than replaced. Where the container
    /// states one it is the better answer, and where it does not the one the
    /// player measured the first time somebody watched is kept.
    pub fn record(&self, films: &[(i64, &MediaDetails)]) -> Result<usize> {
        if films.is_empty() {
            return Ok(0);
        }

        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            for (film_id, details) in films {
                record_one(&transaction, *film_id, details)?;
            }
            transaction.commit()?;
            Ok(films.len())
        })
    }

    /// The sound tracks of one film, in the order the container numbers them.
    pub fn audio(&self, film_id: i64) -> Result<Vec<AudioDetails>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT stream_number, codec, channels, language, is_default
                 FROM audio_stream WHERE film_id = ?1 ORDER BY stream_number",
            )?;
            let tracks = statement
                .query_map([film_id], |row| {
                    Ok(AudioDetails {
                        stream_number: from_sql_int(row.get(0)?),
                        codec: row.get(1)?,
                        channels: row
                            .get::<_, Option<i64>>(2)?
                            .and_then(|channels| u8::try_from(channels).ok()),
                        language: row.get(3)?,
                        default: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(tracks)
        })
    }

    /// The sound tracks of every film, by film.
    ///
    /// One query for the whole library rather than one per film. The library
    /// screen reads every film at once and most of them carry a track or two,
    /// so asking film by film would be several thousand statements to answer a
    /// question one statement answers.
    pub fn all_audio(&self) -> Result<HashMap<i64, Vec<AudioDetails>>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT film_id, stream_number, codec, channels, language, is_default
                 FROM audio_stream ORDER BY film_id, stream_number",
            )?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        AudioDetails {
                            stream_number: from_sql_int(row.get(1)?),
                            codec: row.get(2)?,
                            channels: row
                                .get::<_, Option<i64>>(3)?
                                .and_then(|channels| u8::try_from(channels).ok()),
                            language: row.get(4)?,
                            default: row.get(5)?,
                        },
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            let mut by_film: HashMap<i64, Vec<AudioDetails>> = HashMap::new();
            for (film_id, track) in rows {
                by_film.entry(film_id).or_default().push(track);
            }
            Ok(by_film)
        })
    }
}

/// One film, against a connection already inside a transaction.
fn record_one(connection: &Connection, film_id: i64, details: &MediaDetails) -> Result<()> {
    let video = details.video.as_ref();
    connection
        .prepare_cached(
            "UPDATE film SET
                 container = ?2,
                 video_codec = ?3,
                 video_width = ?4,
                 video_height = ?5,
                 bit_depth = ?6,
                 frame_rate = ?7,
                 duration_ms = coalesce(?8, duration_ms)
             WHERE id = ?1",
        )?
        .execute(params![
            film_id,
            details.container,
            video.map(|video| video.codec.as_str()),
            video.and_then(|video| video.width),
            video.and_then(|video| video.height),
            video.and_then(|video| video.bit_depth),
            video.and_then(|video| video.frame_rate),
            details.duration.map(Timestamp::millis),
        ])?;

    // The film's sound is replaced rather than merged. A re-encode is the case
    // this exists for, and a track it no longer carries has to go rather than
    // sit beside the ones it does.
    connection
        .prepare_cached("DELETE FROM audio_stream WHERE film_id = ?1")?
        .execute([film_id])?;

    let mut insert = connection.prepare_cached(
        "INSERT INTO audio_stream
             (film_id, stream_number, codec, channels, language, is_default)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for track in &details.audio {
        insert.execute(params![
            film_id,
            to_sql_int(track.stream_number),
            track.codec,
            track.channels,
            track.language,
            track.default,
        ])?;
    }
    Ok(())
}
