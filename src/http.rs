use super::*;
use std::ops::Deref;

/// HTTP method of a Steam HTTP request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// `GET`. GET parameters are added to the URL.
    Get,
    /// `HEAD`. The response has no body.
    Head,
    /// `POST`. Parameters are sent form-encoded, or a raw body is sent with a content type.
    Post,
    /// `PUT`.
    Put,
    /// `DELETE`.
    Delete,
    /// `OPTIONS`.
    Options,
    /// `PATCH`.
    Patch,
}

impl From<HttpMethod> for sys::EHTTPMethod {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => sys::EHTTPMethod::k_EHTTPMethodGET,
            HttpMethod::Head => sys::EHTTPMethod::k_EHTTPMethodHEAD,
            HttpMethod::Post => sys::EHTTPMethod::k_EHTTPMethodPOST,
            HttpMethod::Put => sys::EHTTPMethod::k_EHTTPMethodPUT,
            HttpMethod::Delete => sys::EHTTPMethod::k_EHTTPMethodDELETE,
            HttpMethod::Options => sys::EHTTPMethod::k_EHTTPMethodOPTIONS,
            HttpMethod::Patch => sys::EHTTPMethod::k_EHTTPMethodPATCH,
        }
    }
}

/// Access to the steam HTTP interface
pub struct Http {
    pub(crate) http: *mut sys::ISteamHTTP,
    pub(crate) inner: Arc<Inner>,
}

impl Http {
    /// Creates a request for `method` and the absolute `url`, which must start with `http://` or `https://`.
    ///
    /// Nothing is sent yet. Set the request up with the builder methods, then call [`HttpRequest::send`].
    /// Returns `Err(())` when the URL contains a NUL byte or Steam returns no handle.
    ///
    /// Requests go through the Steam client, so they need a running, logged-in client.
    /// Whether HTTPS certificates are checked the same way outside a live client is unverified against a live Steam client.
    pub fn create_request(&self, method: HttpMethod, url: &str) -> Result<HttpRequest, ()> {
        let url = CString::new(url).map_err(|_| ())?;
        let handle = unsafe {
            sys::SteamAPI_ISteamHTTP_CreateHTTPRequest(self.http, method.into(), url.as_ptr())
        };
        // 0 is INVALID_HTTPREQUEST_HANDLE
        if handle == 0 {
            return Err(());
        }
        Ok(HttpRequest {
            handle: Arc::new(RequestHandle {
                http: self.http,
                handle,
                _inner: self.inner.clone(),
            }),
        })
    }

    /// Creates a cookie container that requests can share. It lives for the process only; Steam does not persist its cookies.
    ///
    /// When `allow_responses_to_modify` is `true`, responses to requests that use the container may add cookies that are sent with later requests.
    /// When it is `false`, only cookies set with [`HttpCookieContainer::set_cookie`] are sent.
    /// The container is released when it is dropped. Returns `Err(())` when Steam returns no handle.
    pub fn create_cookie_container(
        &self,
        allow_responses_to_modify: bool,
    ) -> Result<HttpCookieContainer, ()> {
        let handle = unsafe {
            sys::SteamAPI_ISteamHTTP_CreateCookieContainer(self.http, allow_responses_to_modify)
        };
        // 0 is INVALID_HTTPCOOKIE_HANDLE
        if handle == 0 {
            return Err(());
        }
        Ok(HttpCookieContainer {
            http: self.http,
            handle,
        })
    }
}

/// Owns one Steam request handle and releases it with `ReleaseHTTPRequest` when the last owner is dropped.
///
/// An in-flight request keeps its owners alive until its completion callback has run, so Steam never sees the handle released early.
struct RequestHandle {
    http: *mut sys::ISteamHTTP,
    handle: sys::HTTPRequestHandle,
    _inner: Arc<Inner>,
}

impl Drop for RequestHandle {
    fn drop(&mut self) {
        unsafe {
            sys::SteamAPI_ISteamHTTP_ReleaseHTTPRequest(self.http, self.handle);
        }
    }
}

// The handle is an opaque id and Steam's HTTP interface is called from the thread that owns the client.
unsafe impl Send for RequestHandle {}
unsafe impl Sync for RequestHandle {}

/// One HTTP request, from creation until its handle is released.
///
/// The request handle is released (`ReleaseHTTPRequest`) when this value and every [`HttpResponse`] made from it are dropped.
/// Setters take `self` and return `Err(())` when Steam refuses the value. The request is then dropped, which releases it.
pub struct HttpRequest {
    handle: Arc<RequestHandle>,
}

impl HttpRequest {
    /// The raw Steam handle. It matches the `request` field of the streaming callbacks.
    pub fn raw_handle(&self) -> sys::HTTPRequestHandle {
        self.handle.handle
    }

    fn check(self, ok: bool) -> Result<Self, ()> {
        if ok {
            Ok(self)
        } else {
            Err(())
        }
    }

    /// Sets a request header. Must be called before the request is sent.
    ///
    /// Returns `Err(())` when the name or value contains a NUL byte, the handle is invalid, or the request was already sent.
    pub fn header(self, name: &str, value: &str) -> Result<Self, ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let value = CString::new(value).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestHeaderValue(
                self.handle.http,
                self.handle.handle,
                name.as_ptr(),
                value.as_ptr(),
            )
        };
        self.check(ok)
    }

    /// Adds a GET or POST parameter. Steam uses GET for `GET` requests (added to the URL) and POST otherwise (form-encoded).
    ///
    /// Must be called before the request is sent. Fails when the request already has a raw POST body, or when a name or value contains a NUL byte.
    pub fn get_or_post_parameter(self, name: &str, value: &str) -> Result<Self, ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let value = CString::new(value).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestGetOrPostParameter(
                self.handle.http,
                self.handle.handle,
                name.as_ptr(),
                value.as_ptr(),
            )
        };
        self.check(ok)
    }

    /// Sets the whole body of a `POST` request and its `Content-Type`. The body is copied.
    ///
    /// Only valid for POST. Fails when the request has POST parameters already, when the method is not POST, or when the content type contains a NUL byte.
    pub fn raw_post_body(self, content_type: &str, body: &[u8]) -> Result<Self, ()> {
        let content_type = CString::new(content_type).map_err(|_| ())?;
        let mut body = body.to_vec();
        let len = u32::try_from(body.len()).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestRawPostBody(
                self.handle.http,
                self.handle.handle,
                content_type.as_ptr(),
                body.as_mut_ptr(),
                len,
            )
        };
        self.check(ok)
    }

    /// Sets how long the request may go without network activity before it times out. The SDK default is 60 seconds.
    ///
    /// Must be called before the request is sent. The timeout is reset every time data arrives; use [`HttpRequest::absolute_timeout`] to cap the total time.
    pub fn network_activity_timeout(self, seconds: u32) -> Result<Self, ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestNetworkActivityTimeout(
                self.handle.http,
                self.handle.handle,
                seconds,
            )
        };
        self.check(ok)
    }

    /// Sets a limit on the total time of the request, in milliseconds. Unlike the network activity timeout, it does not reset when data arrives.
    ///
    /// Must be called before the request is sent.
    pub fn absolute_timeout(self, milliseconds: u32) -> Result<Self, ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestAbsoluteTimeoutMS(
                self.handle.http,
                self.handle.handle,
                milliseconds,
            )
        };
        self.check(ok)
    }

    /// Adds text to the end of the user agent. The normal user agent is kept.
    ///
    /// Must be called before the request is sent. Fails when `info` contains a NUL byte.
    pub fn user_agent_info(self, info: &str) -> Result<Self, ()> {
        let info = CString::new(info).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestUserAgentInfo(
                self.handle.http,
                self.handle.handle,
                info.as_ptr(),
            )
        };
        self.check(ok)
    }

    /// Turns certificate checking on or off for HTTPS. Checking is on by default.
    ///
    /// Must be called before the request is sent. Whether turning it off has any effect through the Steam client is unverified against a live Steam client.
    pub fn requires_verified_certificate(self, required: bool) -> Result<Self, ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestRequiresVerifiedCertificate(
                self.handle.http,
                self.handle.handle,
                required,
            )
        };
        self.check(ok)
    }

    /// Uses `container` for this request, so cookies are sent and, when the container allows it, stored.
    ///
    /// Must be called before the request is sent.
    pub fn cookie_container(self, container: &HttpCookieContainer) -> Result<Self, ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestCookieContainer(
                self.handle.http,
                self.handle.handle,
                container.handle,
            )
        };
        self.check(ok)
    }

    /// Sets a value that Steam returns in the completion callback and streaming callbacks, so you can match them to this request.
    ///
    /// Must be called before the request is sent. It is not a request handle; the value is only passed back.
    pub fn context_value(self, value: u64) -> Result<Self, ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetHTTPRequestContextValue(
                self.handle.http,
                self.handle.handle,
                value,
            )
        };
        self.check(ok)
    }

    /// Sends the request. `callback` runs once with the completed [`HttpResponse`], from `run_callbacks()`.
    ///
    /// `Err(())` is returned at once when Steam refuses the request, for example for a bad handle. Otherwise the callback gets `Err(SteamError::IOFailure)` when Steam could not complete the call.
    /// A response with `request_successful() == false` means no answer came from the server, for example a network failure or a timeout.
    /// If the Steam client is in offline mode, Steam answers from its local cache only.
    pub fn send<F>(&self, callback: F) -> Result<(), ()>
    where
        F: FnOnce(Result<HttpResponse, SteamError>) + 'static + Send,
    {
        self.send_inner(false, callback)
    }

    /// Sends the request and streams the response. Listen for [`HttpRequestHeadersReceived`] and [`HttpRequestDataReceived`] with `register_callback` while it runs.
    ///
    /// `callback` still runs once when the response is complete, as with [`HttpRequest::send`]. Read streamed chunks with [`HttpRequest::streaming_body_data`], because the body is not kept for later reads.
    pub fn send_and_stream_response<F>(&self, callback: F) -> Result<(), ()>
    where
        F: FnOnce(Result<HttpResponse, SteamError>) + 'static + Send,
    {
        self.send_inner(true, callback)
    }

    fn send_inner<F>(&self, stream: bool, callback: F) -> Result<(), ()>
    where
        F: FnOnce(Result<HttpResponse, SteamError>) + 'static + Send,
    {
        let mut api_call: sys::SteamAPICall_t = 0;
        let ok = unsafe {
            if stream {
                sys::SteamAPI_ISteamHTTP_SendHTTPRequestAndStreamResponse(
                    self.handle.http,
                    self.handle.handle,
                    &mut api_call,
                )
            } else {
                sys::SteamAPI_ISteamHTTP_SendHTTPRequest(
                    self.handle.http,
                    self.handle.handle,
                    &mut api_call,
                )
            }
        };
        if !ok {
            return Err(());
        }

        let request = HttpRequest {
            handle: self.handle.clone(),
        };
        unsafe {
            register_call_result::<sys::HTTPRequestCompleted_t, _>(
                &self.handle._inner,
                api_call,
                move |v, io_error| {
                    if io_error {
                        callback(Err(SteamError::IOFailure));
                    } else {
                        callback(Ok(HttpResponse {
                            request,
                            status_code: v.m_eStatusCode as u32,
                            request_successful: v.m_bRequestSuccessful,
                        }));
                    }
                },
            );
        }
        Ok(())
    }

    /// Moves the request to the back of Steam's queue. Only valid once the request has been sent.
    ///
    /// Returns `Err(())` when the handle is invalid or the request is not sent yet.
    pub fn defer(&self) -> Result<(), ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_DeferHTTPRequest(self.handle.http, self.handle.handle)
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Moves the request to the front of Steam's queue. Only valid once the request has been sent.
    ///
    /// Returns `Err(())` when the handle is invalid or the request is not sent yet.
    pub fn prioritize(&self) -> Result<(), ()> {
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_PrioritizeHTTPRequest(self.handle.http, self.handle.handle)
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Returns the size in bytes of a response header value, or `None` when the header is absent, the name contains a NUL byte, or the handle is invalid.
    ///
    /// Read the value with [`HttpRequest::response_header_value`]. Call it only after the completion callback.
    pub fn response_header_size(&self, name: &str) -> Option<u32> {
        let name = CString::new(name).ok()?;
        let mut size: u32 = 0;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPResponseHeaderSize(
                self.handle.http,
                self.handle.handle,
                name.as_ptr(),
                &mut size,
            )
        };
        if ok {
            Some(size)
        } else {
            None
        }
    }

    /// Returns the value of a response header, or `None` when the header is absent, the name contains a NUL byte, or the handle is invalid.
    ///
    /// Bytes that are not valid UTF-8 are replaced with U+FFFD.
    pub fn response_header_value(&self, name: &str) -> Option<String> {
        let size = self.response_header_size(name)?;
        let name = CString::new(name).ok()?;
        let mut buffer = vec![0u8; size as usize];
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPResponseHeaderValue(
                self.handle.http,
                self.handle.handle,
                name.as_ptr(),
                buffer.as_mut_ptr(),
                size,
            )
        };
        if !ok {
            return None;
        }
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        Some(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }

    /// Returns the size in bytes of the response body, or `None` when the handle is invalid.
    pub fn response_body_size(&self) -> Option<u32> {
        let mut size: u32 = 0;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPResponseBodySize(
                self.handle.http,
                self.handle.handle,
                &mut size,
            )
        };
        if ok {
            Some(size)
        } else {
            None
        }
    }

    /// Returns a copy of the response body, or `None` when the handle is invalid or the response was streamed.
    pub fn response_body_data(&self) -> Option<Vec<u8>> {
        let size = self.response_body_size()?;
        if size == 0 {
            return Some(Vec::new());
        }
        let mut buffer = vec![0u8; size as usize];
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPResponseBodyData(
                self.handle.http,
                self.handle.handle,
                buffer.as_mut_ptr(),
                size,
            )
        };
        if ok {
            Some(buffer)
        } else {
            None
        }
    }

    /// Returns a chunk of a streamed body, from `offset` for `size` bytes, as given by a [`HttpRequestDataReceived`] callback.
    ///
    /// Returns `None` when the handle is invalid, the response was not streamed, or `offset` and `size` do not match a chunk Steam reported.
    pub fn streaming_body_data(&self, offset: u32, size: u32) -> Option<Vec<u8>> {
        if size == 0 {
            return Some(Vec::new());
        }
        let mut buffer = vec![0u8; size as usize];
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPStreamingResponseBodyData(
                self.handle.http,
                self.handle.handle,
                offset,
                buffer.as_mut_ptr(),
                size,
            )
        };
        if ok {
            Some(buffer)
        } else {
            None
        }
    }

    /// Returns how much of the body has been downloaded, from 0 to 100.
    ///
    /// Steam reports 0 until a header with a `Content-Length` has arrived. For responses without a length it stays 0 until the connection closes.
    pub fn download_progress_pct(&self) -> Option<f32> {
        let mut pct: f32 = 0.0;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPDownloadProgressPct(
                self.handle.http,
                self.handle.handle,
                &mut pct,
            )
        };
        if ok {
            Some(pct)
        } else {
            None
        }
    }

    /// Returns whether the request failed because it timed out, rather than for another reason. `None` when Steam does not know the request.
    pub fn was_timed_out(&self) -> Option<bool> {
        let mut timed_out = false;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_GetHTTPRequestWasTimedOut(
                self.handle.http,
                self.handle.handle,
                &mut timed_out,
            )
        };
        if ok {
            Some(timed_out)
        } else {
            None
        }
    }
}

/// The completed response to an [`HttpRequest`], given to the callback of [`HttpRequest::send`].
///
/// It derefs to [`HttpRequest`], so the header, body and timeout getters work on it too.
/// Dropping it releases the request handle, unless the request is still referenced elsewhere.
pub struct HttpResponse {
    request: HttpRequest,
    status_code: u32,
    request_successful: bool,
}

impl HttpResponse {
    /// The HTTP status code the server returned, or 0 when no answer arrived.
    pub fn status_code(&self) -> u32 {
        self.status_code
    }

    /// `true` when any answer arrived from the server, even an error status. `false` when the request failed without an answer (for example a network error or a timeout).
    pub fn request_successful(&self) -> bool {
        self.request_successful
    }
}

impl Deref for HttpResponse {
    type Target = HttpRequest;

    fn deref(&self) -> &HttpRequest {
        &self.request
    }
}

/// A cookie container for HTTP requests. Releases itself with `ReleaseCookieContainer` when dropped.
pub struct HttpCookieContainer {
    http: *mut sys::ISteamHTTP,
    handle: sys::HTTPCookieContainerHandle,
}

unsafe impl Send for HttpCookieContainer {}
unsafe impl Sync for HttpCookieContainer {}

impl HttpCookieContainer {
    /// Adds a cookie that future requests using this container send to `url`.
    ///
    /// `host` is the host the cookie belongs to, and `cookie` is a `Set-Cookie`-style value (`name=value; ...`).
    /// Returns `Err(())` when an argument contains a NUL byte or Steam refuses the cookie.
    pub fn set_cookie(&self, host: &str, url: &str, cookie: &str) -> Result<(), ()> {
        let host = CString::new(host).map_err(|_| ())?;
        let url = CString::new(url).map_err(|_| ())?;
        let cookie = CString::new(cookie).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamHTTP_SetCookie(
                self.http,
                self.handle,
                host.as_ptr(),
                url.as_ptr(),
                cookie.as_ptr(),
            )
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }
}

impl Drop for HttpCookieContainer {
    fn drop(&mut self) {
        unsafe {
            sys::SteamAPI_ISteamHTTP_ReleaseCookieContainer(self.http, self.handle);
        }
    }
}

/// Fired when the response headers of a streaming request have arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpRequestHeadersReceived {
    /// The raw request handle, as [`HttpRequest::raw_handle`] returns it.
    pub request: sys::HTTPRequestHandle,
    /// The context value set with [`HttpRequest::context_value`], or 0.
    pub context_value: u64,
}

impl_callback!(cb: HTTPRequestHeadersReceived_t => HttpRequestHeadersReceived {
    Self {
        request: cb.m_hRequest,
        context_value: cb.m_ulContextValue,
    }
});

/// Fired when a chunk of a streaming response body has arrived. Read it with [`HttpRequest::streaming_body_data`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpRequestDataReceived {
    /// The raw request handle, as [`HttpRequest::raw_handle`] returns it.
    pub request: sys::HTTPRequestHandle,
    /// The context value set with [`HttpRequest::context_value`], or 0.
    pub context_value: u64,
    /// The offset to pass to [`HttpRequest::streaming_body_data`].
    pub offset: u32,
    /// The number of bytes to pass to [`HttpRequest::streaming_body_data`].
    pub bytes_received: u32,
}

impl_callback!(cb: HTTPRequestDataReceived_t => HttpRequestDataReceived {
    Self {
        request: cb.m_hRequest,
        context_value: cb.m_ulContextValue,
        offset: cb.m_cOffset,
        bytes_received: cb.m_cBytesReceived,
    }
});

/// The fields of `HTTPRequestCompleted_t`. Steam reports a completed request through the call result that [`HttpRequest::send`] uses.
/// Whether Steam also dispatches this as a plain callback is unverified against a live Steam client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpRequestCompleted {
    /// The raw request handle, as [`HttpRequest::raw_handle`] returns it.
    pub request: sys::HTTPRequestHandle,
    /// The context value set with [`HttpRequest::context_value`], or 0.
    pub context_value: u64,
    /// `true` when any answer arrived from the server.
    pub request_successful: bool,
    /// The HTTP status code the server returned, or 0.
    pub status_code: u32,
    /// The size of the response body in bytes.
    pub body_size: u32,
}

impl_callback!(cb: HTTPRequestCompleted_t => HttpRequestCompleted {
    Self {
        request: cb.m_hRequest,
        context_value: cb.m_ulContextValue,
        request_successful: cb.m_bRequestSuccessful,
        status_code: cb.m_eStatusCode as u32,
        body_size: cb.m_unBodySize,
    }
});
