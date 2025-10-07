use std::collections::VecDeque;
use std::time::Duration;

use generational_arena::Arena;
use generational_arena::Index;
use tokio::time::Instant;

use crate::TimerCallback;

/// The [`Timers`](crate::Timers) module's internal state.
pub struct TimersState {
    /// Indexes into `timer` sorted by  their `Timer.next` field
    ///
    /// They are sorted such that the **smallest** timestamp is **first**.
    ///
    /// A timer might have been deleted while its index is still in this list.
    /// Such entries should be considered non-existent and skipped until the next valid entry.
    sorted_by_next: VecDeque<Index>,

    /// All scheduled timers
    ///
    /// Each element must have its index in `sorted_by_next`
    timers: Arena<Timer>,
}

/// Public key used to identify a timer
pub struct TimerKey {
    #[expect(dead_code, reason = "TODO")]
    index: Index,
}

struct Timer {
    /// When to trigger this timer next
    next: Instant,

    /// Callback to invoke when the timer is due
    callback: Box<dyn TimerCallback>,

    /// Computes when the time should be run again after its current run.
    schedule: TimerSchedule,
}

/// Computes when a [`Timer`] should be run again after its current run.
enum TimerSchedule {
    /// Schedules the `Timer` to run after a fixed interval of timer after its last run
    Every(Duration),
}

impl TimersState {
    /// Constructs a new (empty) `TimersState`
    pub fn new() -> TimersState {
        Self {
            sorted_by_next: VecDeque::new(),
            timers: Arena::new(),
        }
    }

    /// Schedules `callback` to run every `duration`
    pub fn schedule_every(&mut self, duration: Duration, callback: impl TimerCallback) -> TimerKey {
        self.add(Timer {
            next: Instant::now(),
            callback: Box::new(callback),
            schedule: TimerSchedule::Every(duration),
        })
    }

    /// Returns the next time when some event triggers
    ///
    /// # `None`
    /// if no timer is scheduled
    pub fn next_time(&self) -> Option<Instant> {
        for index in &self.sorted_by_next {
            let Some(timer) = self.timers.get(*index) else {
                // Skip outdated index
                continue;
            };
            return Some(timer.next);
        }
        None
    }

    /// Runs the callbacks of every timer which became due since the last time this method was called and `now`.
    pub fn run(&mut self, now: Instant) {
        // Tracks whether we updated any timer
        let mut any = false;

        for index in &self.sorted_by_next {
            let Some(timer) = self.timers.get_mut(*index) else {
                // Skip outdated index
                continue;
            };
            if timer.next > now {
                // All timers which became due have been processed
                break;
            }

            any = true;
            timer.callback.call();
            timer.next = timer.schedule.calc_next(now);
        }

        if any {
            // Necessary, we updated some timers' `next` field
            self.resort_and_clean();
        }
    }

    /// Adds a new timer
    ///
    /// This method handles recalculating the `sorted_by_next`.
    fn add(&mut self, timer: Timer) -> TimerKey {
        let index = self.timers.insert(timer);
        self.sorted_by_next.push_front(index);

        // Necessary, because we inserted a new timer
        self.resort_and_clean();

        TimerKey { index }
    }

    /// Resorts the `sorted_by_next` field and removes outdated indexes
    ///
    /// This must be called after inserting into `timers` or after updating any `Timer.next`.
    ///
    /// Removing a value from `timers` does not require a call to this method.
    fn resort_and_clean(&mut self) {
        assert!(
            self.sorted_by_next.len() >= self.timers.len(),
            "There should never be more timers than indexes"
        );

        // Resort all indexes
        //
        // The sorting key is `Option<Instant>`.
        // It is `None` for an index whose timer has been deleted.
        // `None` is smaller than `Some(_)`.
        // => All outdated indexes will be at the front after sorting
        self.sorted_by_next
            .make_contiguous()
            .sort_unstable_by_key(|index| self.timers.get(*index).map(|x| x.next));

        // Pop all outdated indexes from the front
        while let Some(last_index) = self.sorted_by_next.front().copied() {
            if self.timers.contains(last_index) {
                break;
            } else {
                self.sorted_by_next.pop_front();
            }
        }

        assert_eq!(
            self.sorted_by_next.len(),
            self.timers.len(),
            "The number indexes and timers should match after cleaning the outdated indexes"
        );
    }
}

impl TimerSchedule {
    /// Computes a [`Timer`]'s `next` field
    pub fn calc_next(&mut self, current_run: Instant) -> Instant {
        match self {
            TimerSchedule::Every(duration) => current_run + *duration,
        }
    }
}
