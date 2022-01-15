#[cfg_attr(test, derive(Debug))]
pub(crate) struct Headers {
    values: Vec<u8>,
    parts: Vec<HeaderPart>,
}

#[cfg_attr(test, derive(Debug))]
struct HeaderPart {
    name: &'static str,
    start: usize,
    end: usize,
}

impl Headers {
    #[inline]
    pub(crate) fn empty() -> Self {
        Self {
            values: vec![],
            parts: vec![],
        }
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.parts.len()
    }

    #[inline]
    pub(crate) fn get(&self, name: &'static str) -> Option<&[u8]> {
        self.parts
            .iter()
            .find(|p| p.name == name)
            .map(|p| &self.values[p.start..p.end])
    }

    #[inline]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'static str, &[u8])> {
        self.parts
            .iter()
            .map(|p| (p.name, &self.values[p.start..p.end]))
    }
}

impl From<&mut [httparse::Header<'_>]> for Headers {
    #[inline]
    fn from(httparse_headers: &mut [httparse::Header<'_>]) -> Self {
        let values_len = httparse_headers.iter().map(|h| h.value.len()).sum();
        let mut headers = Headers {
            values: Vec::with_capacity(values_len),
            parts: Vec::with_capacity(httparse_headers.len()),
        };
        let mut start = 0;
        httparse_headers
            .iter()
            .map(|h| (h.name, h.value))
            .for_each(|(n, v)| {
                let name = match n {
                    "content-length" => "content-length",
                    "date" => "date",
                    n => {
                        tracing::warn!("ignoring unsupported header: {}", n);
                        return;
                    }
                };
                headers.parts.push(HeaderPart {
                    name,
                    start,
                    end: start + v.len(),
                });
                start += v.len();
                headers.values.extend_from_slice(v);
            });
        headers
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
impl From<hyper::HeaderMap<hyper::header::HeaderValue>> for Headers {
    #[inline]
    fn from(hyper_headers: hyper::HeaderMap<hyper::header::HeaderValue>) -> Self {
        let values_len = hyper_headers.iter().map(|(n, v)| v.len()).sum();
        let mut headers = Headers {
            values: Vec::with_capacity(values_len),
            parts: Vec::with_capacity(hyper_headers.len()),
        };
        let mut start = 0;
        hyper_headers.iter().for_each(|(n, v)| {
            let name = match n {
                &hyper::http::header::CONTENT_LENGTH => "content-length",
                &hyper::http::header::DATE => "date",
                n => {
                    tracing::warn!("ignoring unsupported header: {}", n);
                    return;
                }
            };
            headers.parts.push(HeaderPart {
                name,
                start,
                end: start + v.len(),
            });
            start += v.len();
            headers.values.extend_from_slice(v.as_bytes());
        });
        headers
    }
}
