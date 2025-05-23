use std::future::poll_fn;
use std::future::Future;
use std::io;
use std::pin::Pin;

use futures_lite::Stream;
use signal_hook_tokio::Signals;
use tracing::debug;
use tracing::warn;

/// Constructs a future with resolves after receiving a [termination signal](signal_hook::consts::TERM_SIGNALS)
///
/// # Errors
/// if the signal handler can't be registered
pub fn wait_for_signal() -> io::Result<impl Future<Output = ()>> {
    let mut signals = Signals::new(signal_hook::consts::TERM_SIGNALS)?;
    Ok(async move {
        let handle = signals.handle();
        let signal = poll_fn(|ctx| Pin::new(&mut signals).poll_next(ctx)).await;
        match signal {
            Some(sig_num) => debug!(signal.number = sig_num, "Shutting down"),
            None => warn!("Signal stream terminated, this is unexpected!"),
        }
        handle.close();
    })
}
