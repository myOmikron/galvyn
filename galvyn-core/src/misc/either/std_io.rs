//! `std::io` implementations for `Either`

use std::fmt::Arguments;
use std::io;
use std::io::IoSlice;
use std::io::IoSliceMut;
use std::io::SeekFrom;

use crate::misc::either::Either;

impl<L, R> io::Read for Either<L, R>
where
    L: io::Read,
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.read(buf),
            Either::Right(x) => x.read(buf),
        }
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.read_vectored(bufs),
            Either::Right(x) => x.read_vectored(bufs),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.read_to_end(buf),
            Either::Right(x) => x.read_to_end(buf),
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.read_to_string(buf),
            Either::Right(x) => x.read_to_string(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        match self {
            Either::Left(x) => x.read_exact(buf),
            Either::Right(x) => x.read_exact(buf),
        }
    }
}

impl<L, R> io::Write for Either<L, R>
where
    L: io::Write,
    R: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.write(buf),
            Either::Right(x) => x.write(buf),
        }
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.write_vectored(bufs),
            Either::Right(x) => x.write_vectored(bufs),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Either::Left(x) => x.flush(),
            Either::Right(x) => x.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Either::Left(x) => x.write_all(buf),
            Either::Right(x) => x.write_all(buf),
        }
    }

    fn write_fmt(&mut self, args: Arguments<'_>) -> io::Result<()> {
        match self {
            Either::Left(x) => x.write_fmt(args),
            Either::Right(x) => x.write_fmt(args),
        }
    }
}

impl<L, R> io::Seek for Either<L, R>
where
    L: io::Seek,
    R: io::Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            Either::Left(x) => x.seek(pos),
            Either::Right(x) => x.seek(pos),
        }
    }

    fn rewind(&mut self) -> io::Result<()> {
        match self {
            Either::Left(x) => x.rewind(),
            Either::Right(x) => x.rewind(),
        }
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        match self {
            Either::Left(x) => x.stream_position(),
            Either::Right(x) => x.stream_position(),
        }
    }

    fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        match self {
            Either::Left(x) => x.seek_relative(offset),
            Either::Right(x) => x.seek_relative(offset),
        }
    }
}

impl<L, R> io::BufRead for Either<L, R>
where
    L: io::BufRead,
    R: io::BufRead,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            Either::Left(x) => x.fill_buf(),
            Either::Right(x) => x.fill_buf(),
        }
    }

    fn consume(&mut self, amount: usize) {
        match self {
            Either::Left(x) => x.consume(amount),
            Either::Right(x) => x.consume(amount),
        }
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.read_until(byte, buf),
            Either::Right(x) => x.read_until(byte, buf),
        }
    }

    fn skip_until(&mut self, byte: u8) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.skip_until(byte),
            Either::Right(x) => x.skip_until(byte),
        }
    }

    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        match self {
            Either::Left(x) => x.read_line(buf),
            Either::Right(x) => x.read_line(buf),
        }
    }
}
