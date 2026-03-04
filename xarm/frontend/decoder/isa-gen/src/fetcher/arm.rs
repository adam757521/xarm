use ureq;
use tar::Archive;

use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
enum ContentEncodingMirror {
    None,
    Gzip,
    _Brotli,
    _Unknown,
}

#[repr(C)]
struct ResponseInfoMirror {
    _skip: [u64; 8],
    pub number: ContentEncodingMirror
}

struct BodyMirror {
    _skip_source: [u64; 2],
    pub info: Arc<ResponseInfoMirror>
}

pub struct InstructionSpecificationStream {
    archive: Archive<ureq::BodyReader<'static>>
}

// This pattern would have been more efficient in C++
// There are inefficiencies here
pub struct InstructionSpecificationIter<'archive> {
    iter: tar::Entries<'archive, ureq::BodyReader<'static>>,
    first_folder: PathBuf,
}

impl InstructionSpecificationStream {
    pub fn connect(url: &str) -> Result<Self, ureq::Error> {
        let mut body = ureq::get(url)
            .header("User-Agent", "Python-urllib")
            .call()?
            .into_body();

        // Make ureq decode as gzip
        unsafe {
            let mirrored: &mut BodyMirror = std::mem::transmute(&mut body);
            let data_ptr = std::sync::Arc::as_ptr(&mirrored.info) as *mut ResponseInfoMirror;
            if (*data_ptr).number != ContentEncodingMirror::None {
                return Err(ureq::Error::ConnectProxyFailed("Expected content encoding to be none".to_string()));
            }
            (*data_ptr).number = ContentEncodingMirror::Gzip;
        }

        Ok(Self { 
            archive: Archive::new(body.into_reader())
        })
    }

    pub fn make_iter(&mut self) -> Option<InstructionSpecificationIter<'_>> {
        let mut iter = self.archive.entries().ok()?;

        let first_folder_entry = iter.next()?.ok()?;
        // Heap allocation
        let first_folder = first_folder_entry.header().path().ok()?.to_path_buf();

        Some(InstructionSpecificationIter {
            iter,
            first_folder
        })
    }
}

impl<'archive> InstructionSpecificationIter<'archive>{
    pub fn next(&mut self) -> Option<tar::Entry<'archive, ureq::BodyReader<'static>>> { 
        while let Some(Ok(entry)) = self.iter.next() {
            let path = entry.path().ok()?;
            let parent = path.parent()?;

            if self.first_folder != parent {
                return None;
            }

            if path.extension().map_or(false, |ext| ext == "xml") {
                return Some(entry);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_specification_iter() {
        const ARM_SPEC: &str = "https://developer.arm.com/-/cdn-downloads/permalink/Exploration-Tools-A64-ISA/ISA_A64/ISA_A64_xml_A_profile-2025-06.tar.gz";

        let mut stream = InstructionSpecificationStream::connect(ARM_SPEC)
            .unwrap();

        stream
            .make_iter()
            .unwrap();

        /*
        it.next_with(|e| {
            assert!(e.path().unwrap().extension().is_some())
        });*/
    }
}
