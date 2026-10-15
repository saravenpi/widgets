//! Test-only helpers shared by the widget checks in this binary.
//!
//! GTK may only be used from the thread that initialised it, and the checks in
//! one binary run in parallel on threads of their own, so a second check reaching
//! for the toolkit from its own thread is not a red test but a panic inside
//! gtk4-rs ("GTK may only be used from the main thread"). The widget checks hand
//! their body to [`on_gtk_thread`] instead of calling GTK directly.
//!
//! gtk4-rs ships `#[gtk::test]` for exactly this, but it *panics* when GTK cannot
//! come up rather than reporting a display-less machine, and every widget check
//! here has to stand down quietly where there is none. So the thread is ours.

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;

/// One check for the toolkit thread to run, and where to send what came of it.
struct Job {
    check: Box<dyn FnOnce() + Send>,
    done: Sender<std::thread::Result<()>>,
}

/// The toolkit thread, or `None` when GTK could not come up because there is no
/// display. Started by the first widget check that asks.
static GTK: OnceLock<Option<Sender<Job>>> = OnceLock::new();

/// Run `check` on the thread GTK was initialised on, and report whether it ran.
///
/// The bodies run one at a time on that single thread, which is also what keeps
/// two of them from building widgets at the same moment — GTK is not thread safe
/// either, so serialising and owning the thread are the same job.
///
/// `false` means there is no display and `check` never ran, which is how a check
/// stands down on a machine that cannot draw. A failed assertion inside `check`
/// is re-raised here, on the test's own thread, with its message intact.
pub fn on_gtk_thread(check: impl FnOnce() + Send + 'static) -> bool {
    let Some(jobs) = GTK.get_or_init(start) else {
        return false;
    };
    let (done, outcome) = channel();
    jobs.send(Job {
        check: Box::new(check),
        done,
    })
    .expect("the toolkit thread is alive");
    match outcome.recv() {
        Ok(Ok(())) => true,
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("the toolkit thread went away"),
    }
}

/// Start the thread GTK lives on: initialise the toolkit once, then run every
/// check it is handed, in turn.
fn start() -> Option<Sender<Job>> {
    let (jobs, queue) = channel::<Job>();
    let (ready, started) = channel::<bool>();
    std::thread::spawn(move || {
        let usable = gtk4::init().is_ok() && gtk4::gdk::Display::default().is_some();
        let _ = ready.send(usable);
        if !usable {
            return;
        }
        for job in queue {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job.check));
            let _ = job.done.send(outcome);
        }
    });
    started.recv().ok().filter(|usable| *usable).map(|_| jobs)
}
