//! [`http_body`] integration.

/// An [`http_body::Body`] reader.
#[derive(Debug)]
pub struct Reader<T>(pub T);

impl<T> crate::Reader for Reader<T>
where
    T: http_body::Body + Unpin,
{
    type Data<'data> = <T as http_body::Body>::Data;
    type Error = <T as http_body::Body>::Error;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        self.0.data().await
    }
}
