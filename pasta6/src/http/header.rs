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
    pub(super) fn len(&self) -> usize {
        self.parts.len()
    }

    #[inline]
    pub(super) fn get(&self, name: &'static str) -> Option<&[u8]> {
        self.parts
            .iter()
            .find(|p| p.name == name)
            .map(|p| &self.values[p.start..p.end])
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
                        tracing::error!("unsupported header: {}", n);
                        unimplemented!()
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
