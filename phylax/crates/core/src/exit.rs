use tokio::sync::oneshot;

use crate::error::PhylaxNodeError;
use futures_util::FutureExt;
use std::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};

/// A Future which resolves when the node exits
#[derive(Debug)]
pub struct PhylaxExitFuture {
    /// The shutdown receiver half of the channel for Phylax execution.
    node_shutdown_rx: Option<oneshot::Receiver<Result<(), PhylaxNodeError>>>,
}

impl PhylaxExitFuture {
    /// Create a new [`PhylaxExitFuture`].
    pub fn new(node_shutdown_rx: oneshot::Receiver<Result<(), PhylaxNodeError>>) -> Self {
        Self { node_shutdown_rx: Some(node_shutdown_rx) }
    }
}

impl Future for PhylaxExitFuture {
    type Output = Result<(), PhylaxNodeError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(rx) = this.node_shutdown_rx.as_mut() {
            match ready!(rx.poll_unpin(cx)) {
                Ok(res) => {
                    // Drop the channel
                    this.node_shutdown_rx.take();
                    // Propagate any errors sent by
                    res?;
                    Poll::Ready(Ok(()))
                }
                Err(err) => Poll::Ready(Err(err.into())),
            }
        } else {
            Poll::Pending
        }
    }
}
