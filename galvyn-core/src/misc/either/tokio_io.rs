//! `tokio::io` implementations for `Either`

use std::io::IoSlice;
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use tokio::io;
use tokio::io::ReadBuf;

use crate::misc::either::Either;

impl<L, R> io::AsyncRead for Either<L, R>
where
    L: io::AsyncRead,
    R: io::AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_read(cx, buf),
            Either::Right(x) => x.poll_read(cx, buf),
        }
    }
}

impl<L, R> io::AsyncWrite for Either<L, R>
where
    L: io::AsyncWrite,
    R: io::AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_write(cx, buf),
            Either::Right(x) => x.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_flush(cx),
            Either::Right(x) => x.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_shutdown(cx),
            Either::Right(x) => x.poll_shutdown(cx),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_write_vectored(cx, bufs),
            Either::Right(x) => x.poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Either::Left(x) => x.is_write_vectored(),
            Either::Right(x) => x.is_write_vectored(),
        }
    }
}

impl<L, R> io::AsyncSeek for Either<L, R>
where
    L: io::AsyncSeek,
    R: io::AsyncSeek,
{
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        match self.as_pin_mut() {
            Either::Left(x) => x.start_seek(position),
            Either::Right(x) => x.start_seek(position),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_complete(cx),
            Either::Right(x) => x.poll_complete(cx),
        }
    }
}

impl<L, R> io::AsyncBufRead for Either<L, R>
where
    L: io::AsyncBufRead,
    R: io::AsyncBufRead,
{
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        match self.as_pin_mut() {
            Either::Left(x) => x.poll_fill_buf(cx),
            Either::Right(x) => x.poll_fill_buf(cx),
        }
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        match self.as_pin_mut() {
            Either::Left(x) => x.consume(amt),
            Either::Right(x) => x.consume(amt),
        }
    }
}
