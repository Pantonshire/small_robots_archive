use std::{future, ops::Deref};

use actix_web::{
    HttpRequest,
    FromRequest,
    error::ErrorInternalServerError,
    dev::Payload,
};

pub struct CloneData<T> where T: Clone + 'static {
    pub inner: T,
}

impl<T> CloneData<T> where T: Clone + 'static {
    pub fn new(val: T) -> Self {
        Self {
            inner: val,
        }
    }
}

impl<T> Clone for CloneData<T> where T: Clone + 'static {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone()
        }
    }
}

impl<T> Deref for CloneData<T> where T: Clone + 'static {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> FromRequest for CloneData<T> where T: Clone + 'static {
    type Config = ();
    type Error = actix_web::error::Error;
    type Future = future::Ready<Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        future::ready(match req.app_data::<Self>() {
            Some(cd) => Ok(cd.clone()),
            None => Err(ErrorInternalServerError("no PgPool found in app data")),
        })
    }
}
