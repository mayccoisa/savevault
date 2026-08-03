use crate::prelude::{Security, get_reqwest_blocking_client, get_reqwest_client};

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Release {
    pub version: semver::Version,
    pub url: String,
    /// Direct link to the Windows build attached to the release, when there is one.
    ///
    /// This is what makes updating one click instead of a trip to the browser.
    pub download: Option<String>,
}

/// The GitHub payload, kept separate from [`Release`] because the two do not have the same shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct ReleaseResponse {
    html_url: String,
    tag_name: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

impl ReleaseResponse {
    fn into_release(self) -> Result<Release, crate::prelude::AnyError> {
        let download = self
            .assets
            .iter()
            .find(|asset| {
                let name = asset.name.to_lowercase();
                name.ends_with(".zip") && name.contains("win")
            })
            .map(|asset| asset.browser_download_url.clone());

        Ok(Release {
            version: Release::parse_tag(&self.tag_name)?,
            url: self.html_url,
            download,
        })
    }
}

impl Release {
    /// SaveVault's own releases, not the upstream project's. Pointing this at Ludusavi would make
    /// the app report an update whose version numbers belong to a different release line.
    const URL: &'static str = "https://api.github.com/repos/mayccoisa/savevault/releases/latest";

    /// SaveVault's tags are `savevault-vX.Y.Z`.
    ///
    /// The prefix is not cosmetic: this repository inherited 55 tags named `v0.1.0` through
    /// `v0.31.0` from Ludusavi, so an unprefixed tag would collide for the next thirty versions.
    /// Trimming only the `v`, as the upstream code did, leaves `savevault-v0.2.0`, which is not a
    /// semantic version, so **every** update check failed silently and the app would never notice
    /// a new release. Old unprefixed tags still parse, so nothing is lost by accepting both.
    fn parse_tag(tag: &str) -> Result<semver::Version, crate::prelude::AnyError> {
        let cleaned = tag.trim_start_matches("savevault-").trim_start_matches('v');
        Ok(semver::Version::parse(cleaned)?)
    }

    pub async fn fetch(security: Security) -> Result<Self, crate::prelude::AnyError> {
        let req = get_reqwest_client(security)
            .get(Self::URL)
            .header(reqwest::header::USER_AGENT, &*crate::prelude::USER_AGENT);
        let res = req.send().await?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let bytes = res.bytes().await?.to_vec();
                let raw = String::from_utf8(bytes)?;
                serde_json::from_str::<ReleaseResponse>(&raw)?.into_release()
            }
            code => Err(format!("status code: {code:?}").into()),
        }
    }

    pub fn fetch_sync(security: Security) -> Result<Self, crate::prelude::AnyError> {
        let req = get_reqwest_blocking_client(security)
            .get(Self::URL)
            .header(reqwest::header::USER_AGENT, &*crate::prelude::USER_AGENT);
        let res = req.send()?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let bytes = res.bytes()?.to_vec();
                let raw = String::from_utf8(bytes)?;
                serde_json::from_str::<ReleaseResponse>(&raw)?.into_release()
            }
            code => Err(format!("status code: {code:?}").into()),
        }
    }

    /// Downloads the new build and puts it in place, so the user only has to restart.
    ///
    /// The running executable cannot be overwritten while it is running, but on Windows it *can*
    /// be renamed, so the old build is moved aside and the new one takes its name. The old file is
    /// left behind on purpose: deleting the binary that is currently executing is the one step
    /// that could leave the user with no working program at all if it went wrong.
    /// The error is a `String` rather than the crate's boxed error on purpose: this runs inside
    /// the GUI's async runtime, which requires the future to be `Send`, and the boxed error is not.
    pub async fn install(&self, security: Security) -> Result<(), String> {
        self.install_inner(security).await.map_err(|e| e.to_string())
    }

    async fn install_inner(&self, security: Security) -> Result<(), crate::prelude::AnyError> {
        let Some(url) = self.download.as_ref() else {
            return Err("this release has no Windows build attached to it".into());
        };

        let res = get_reqwest_client(security)
            .get(url)
            .header(reqwest::header::USER_AGENT, &*crate::prelude::USER_AGENT)
            .send()
            .await?;
        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("could not download the update: {:?}", res.status()).into());
        }
        let bytes = res.bytes().await?.to_vec();

        let current = std::env::current_exe()?;
        let folder = current
            .parent()
            .ok_or("could not determine where this program is installed")?
            .to_path_buf();

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
        let wanted = current
            .file_name()
            .map(|x| x.to_string_lossy().to_lowercase())
            .ok_or("could not determine this program's file name")?;

        let index = (0..archive.len())
            .find(|i| {
                archive
                    .by_index(*i)
                    .ok()
                    .and_then(|entry| {
                        entry
                            .enclosed_name()
                            .and_then(|name| name.file_name().map(|x| x.to_string_lossy().to_lowercase()))
                    })
                    .is_some_and(|name| name == wanted)
            })
            .ok_or("the downloaded archive does not contain the program")?;

        // Written beside the current program, and not to a temporary folder, so that the final
        // step is a rename within one drive: an atomic operation that cannot leave half a file.
        let staged = folder.join(format!("{wanted}.new"));
        {
            let mut entry = archive.by_index(index)?;
            let mut out = std::fs::File::create(&staged)?;
            std::io::copy(&mut entry, &mut out)?;
        }

        let retired = folder.join(format!("{wanted}.old"));
        let _ = std::fs::remove_file(&retired);
        std::fs::rename(&current, &retired)?;

        if let Err(e) = std::fs::rename(&staged, &current) {
            // Put the working program back: a failure here must not leave the user with nothing.
            let _ = std::fs::rename(&retired, &current);
            let _ = std::fs::remove_file(&staged);
            return Err(e.into());
        }

        Ok(())
    }

    pub fn is_update(&self) -> bool {
        if let Ok(current) = semver::Version::parse(*crate::prelude::VERSION) {
            self.version > current
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bug this guards: SaveVault's own tags carry a prefix, and trimming only the `v` left a
    /// string that is not a semantic version, so the update check failed for every real release.
    #[test]
    fn parses_savevault_tags_and_the_inherited_ones() {
        assert_eq!(
            semver::Version::parse("0.2.0").unwrap(),
            Release::parse_tag("savevault-v0.2.0").unwrap()
        );
        assert_eq!(
            semver::Version::parse("0.31.0").unwrap(),
            Release::parse_tag("v0.31.0").unwrap()
        );
        assert!(Release::parse_tag("nightly").is_err());
    }

    #[test]
    fn picks_the_windows_archive_out_of_the_assets() {
        let raw = r#"{
            "html_url": "https://example.com/release",
            "tag_name": "savevault-v0.2.0",
            "assets": [
                {"name": "savevault-v0.2.0-linux.zip", "browser_download_url": "https://example.com/linux.zip"},
                {"name": "savevault-v0.2.0-win64.zip", "browser_download_url": "https://example.com/win.zip"},
                {"name": "notes.txt", "browser_download_url": "https://example.com/notes.txt"}
            ]
        }"#;

        let release = serde_json::from_str::<ReleaseResponse>(raw).unwrap().into_release().unwrap();

        assert_eq!(Some("https://example.com/win.zip".to_string()), release.download);
    }

    /// A release with nothing attached must not pretend it can update itself.
    #[test]
    fn a_release_without_a_build_has_nothing_to_download() {
        let raw = r#"{"html_url": "https://example.com", "tag_name": "savevault-v0.2.0", "assets": []}"#;

        let release = serde_json::from_str::<ReleaseResponse>(raw).unwrap().into_release().unwrap();

        assert_eq!(None, release.download);
    }
}
