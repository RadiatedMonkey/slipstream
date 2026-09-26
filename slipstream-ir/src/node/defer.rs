use slipstream_shared::error::{InvalidInputError, SlipstreamError, SlipstreamResult};

pub const FORCE_EAGER_EVALUATION: bool = false;

/// Possible data passed into the deferred node callback.
pub struct DeferPayload<P, F, T>
where
    F: FnOnce(P) -> SlipstreamResult<T>,
{
    payload: Option<(P, F)>,
}

impl<P, F, T> DeferParser<T> for DeferPayload<P, F, T>
where
    P: Send + Sync,
    T: Send + Sync,
    F: FnOnce(P) -> SlipstreamResult<T> + Send + Sync,
{
    fn load(&mut self) -> SlipstreamResult<T> {
        let (data, parse_fn) = self.payload.take().ok_or_else(|| {
            SlipstreamError::from(InvalidInputError {
                reason: String::from("deferred node has no payload"),
                ..Default::default()
            })
        })?;

        tracing::debug!("Lazily parsing node");

        (parse_fn)(data)
    }
}

/// Implemented by the deferred nodes. This is required to put the callback inside of a
/// box.
pub trait DeferParser<T>: Send + Sync {
    /// Forces the parser to be evaluated, returning the data it produced.
    fn load(&mut self) -> SlipstreamResult<T>;
}

/// A possibly deferred parsing operation.
pub enum Deferred<T> {
    /// The object has not been parsed yet, so no data is available.
    Deferred(Box<dyn DeferParser<T>>),
    /// The object has been fully parsed and the data is available.
    Evaluated(T),
}

impl<T> Deferred<T>
where
    T: Send + Sync + 'static,
{
    /// Creates deferred content.
    ///
    /// In other words, the given `data` will be parsed using the `parse_fn` only
    /// when requested.
    pub fn defer<P, F>(data: P, parse_fn: F) -> SlipstreamResult<Self>
    where
        P: Send + Sync + 'static,
        F: FnOnce(P) -> SlipstreamResult<T> + Send + Sync + 'static,
    {
        if FORCE_EAGER_EVALUATION {
            let eval = parse_fn(data)?;
            return Ok(Self::Evaluated(eval));
        }

        Ok(Self::Deferred(Box::new(DeferPayload {
            payload: Some((data, parse_fn)),
        })))
    }

    /// Runs the given closure, and returns its output, if the object has been evaluated.
    ///
    /// Returns `None` otherwise.
    pub fn inspect_ref<F, O>(&self, peek_fn: F) -> Option<O>
    where
        F: FnOnce(&T) -> O,
    {
        match self {
            Self::Evaluated(x) => Some(peek_fn(x)),
            _ => None,
        }
    }

    /// Runs the given closure, and return its output, if this object's contents have been parsed.
    pub fn inspect_mut<F, O>(&mut self, peek_fn: F) -> Option<O>
    where
        F: FnOnce(&mut T) -> O,
    {
        match self {
            Self::Evaluated(x) => Some(peek_fn(x)),
            _ => None,
        }
    }

    /// Creates a new, already parsed, deferred object.
    pub fn evaluated(data: T) -> Self {
        Self::Evaluated(data)
    }

    /// Forces the current contents to be parsed, returning a mutable reference to the resulting data.
    pub fn evaluate(&mut self) -> SlipstreamResult<&mut T> {
        Ok(match self {
            Self::Evaluated(x) => x,
            Self::Deferred(payload) => {
                let eval = payload.load().unwrap();
                *self = Self::Evaluated(eval);

                let Self::Evaluated(eval) = self else {
                    unreachable!()
                };
                eval
            }
        })
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            Self::Evaluated(x) => Some(x),
            _ => None,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Evaluated(x) => Some(x),
            _ => None,
        }
    }

    /// Whether the inner contents have fully been parsed.
    pub fn is_deferred(&self) -> bool {
        matches!(self, Self::Deferred(_))
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Deferred<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Evaluated(x) => fmt.debug_set().entry(x).finish(),
            Self::Deferred(_) => fmt.debug_set().finish_non_exhaustive(), // can't print a closure, so just print ellipsis.
        }
    }
}
