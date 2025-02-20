/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Configuration options for a single run of the servo application. Created
//! from command line arguments.

use crate::prefs::{self, PrefValue};
use euclid::TypedSize2D;
use getopts::Options;
use servo_geometry::DeviceIndependentPixel;
use servo_url::ServoUrl;
use std::borrow::Cow;
use std::default::Default;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{RwLock, RwLockReadGuard};
use url::{self, Url};

/// Global flags for Servo, currently set on the command line.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Opts {
    pub is_running_problem_test: bool,

    /// The initial URL to load.
    pub url: Option<ServoUrl>,

    /// The maximum size of each tile in pixels (`-s`).
    pub tile_size: usize,

    /// The ratio of device pixels per px at the default scale. If unspecified, will use the
    /// platform default setting.
    pub device_pixels_per_px: Option<f32>,

    /// `None` to disable the time profiler or `Some` to enable it with:
    ///
    ///  - an interval in seconds to cause it to produce output on that interval.
    ///    (`i.e. -p 5`).
    ///  - a file path to write profiling info to a TSV file upon Servo's termination.
    ///    (`i.e. -p out.tsv`).
    ///  - an InfluxDB hostname to store profiling info upon Servo's termination.
    ///    (`i.e. -p http://localhost:8086`)
    pub time_profiling: Option<OutputOptions>,

    /// When the profiler is enabled, this is an optional path to dump a self-contained HTML file
    /// visualizing the traces as a timeline.
    pub time_profiler_trace_path: Option<String>,

    /// `None` to disable the memory profiler or `Some` with an interval in seconds to enable it
    /// and cause it to produce output on that interval (`-m`).
    pub mem_profiler_period: Option<f64>,

    /// True to turn off incremental layout.
    pub nonincremental_layout: bool,

    /// Where to load userscripts from, if any. An empty string will load from
    /// the resources/user-agent-js directory, and if the option isn't passed userscripts
    /// won't be loaded
    pub userscripts: Option<String>,

    pub user_stylesheets: Vec<(Vec<u8>, ServoUrl)>,

    pub output_file: Option<String>,

    /// Replace unpaired surrogates in DOM strings with U+FFFD.
    /// See <https://github.com/servo/servo/issues/6564>
    pub replace_surrogates: bool,

    /// Log GC passes and their durations.
    pub gc_profile: bool,

    /// Load web fonts synchronously to avoid non-deterministic network-driven reflows.
    pub load_webfonts_synchronously: bool,

    pub headless: bool,

    /// Use ANGLE to create the GL context (Windows-only).
    pub angle: bool,

    /// True to exit on thread failure instead of displaying about:failure.
    pub hard_fail: bool,

    /// True if we should bubble intrinsic widths sequentially (`-b`). If this is true, then
    /// intrinsic widths are computed as a separate pass instead of during flow construction. You
    /// may wish to turn this flag on in order to benchmark style recalculation against other
    /// browser engines.
    pub bubble_inline_sizes_separately: bool,

    /// True if we should show borders on all fragments for debugging purposes
    /// (`--show-debug-fragment-borders`).
    pub show_debug_fragment_borders: bool,

    /// True if we should paint borders around flows based on which thread painted them.
    pub show_debug_parallel_layout: bool,

    /// If set with --disable-text-aa, disable antialiasing on fonts. This is primarily useful for reftests
    /// where pixel perfect results are required when using fonts such as the Ahem
    /// font for layout tests.
    pub enable_text_antialiasing: bool,

    /// If set with --disable-subpixel, use subpixel antialiasing for glyphs. In the future
    /// this will likely become the default, but for now it's opt-in while we work
    /// out any bugs and improve the implementation.
    pub enable_subpixel_text_antialiasing: bool,

    /// If set with --disable-canvas-aa, disable antialiasing on the HTML canvas element.
    /// Like --disable-text-aa, this is useful for reftests where pixel perfect results are required.
    pub enable_canvas_antialiasing: bool,

    /// True if each step of layout is traced to an external JSON file
    /// for debugging purposes. Setting this implies sequential layout
    /// and paint.
    pub trace_layout: bool,

    /// Periodically print out on which events script threads spend their processing time.
    pub profile_script_events: bool,

    /// Enable all heartbeats for profiling.
    pub profile_heartbeats: bool,

    /// `None` to disable debugger or `Some` with a port number to start a server to listen to
    /// remote Firefox debugger connections.
    pub debugger_port: Option<u16>,

    /// `None` to disable devtools or `Some` with a port number to start a server to listen to
    /// remote Firefox devtools connections.
    pub devtools_port: Option<u16>,

    /// `None` to disable WebDriver or `Some` with a port number to start a server to listen to
    /// remote WebDriver commands.
    pub webdriver_port: Option<u16>,

    /// The initial requested size of the window.
    pub initial_window_size: TypedSize2D<u32, DeviceIndependentPixel>,

    /// An optional string allowing the user agent to be set for testing.
    pub user_agent: Cow<'static, str>,

    /// Whether we're running in multiprocess mode.
    pub multiprocess: bool,

    /// Whether we're running inside the sandbox.
    pub sandbox: bool,

    /// Probability of randomly closing a pipeline,
    /// used for testing the hardening of the constellation.
    pub random_pipeline_closure_probability: Option<f32>,

    /// The seed for the RNG used to randomly close pipelines,
    /// used for testing the hardening of the constellation.
    pub random_pipeline_closure_seed: Option<usize>,

    /// Dumps the DOM after restyle.
    pub dump_style_tree: bool,

    /// Dumps the rule tree.
    pub dump_rule_tree: bool,

    /// Dumps the flow tree after a layout.
    pub dump_flow_tree: bool,

    /// Dumps the display list after a layout.
    pub dump_display_list: bool,

    /// Dumps the display list in JSON form after a layout.
    pub dump_display_list_json: bool,

    /// Emits notifications when there is a relayout.
    pub relayout_event: bool,

    /// Whether Style Sharing Cache is used
    pub disable_share_style_cache: bool,

    /// Whether to show in stdout style sharing cache stats after a restyle.
    pub style_sharing_stats: bool,

    /// Translate mouse input into touch events.
    pub convert_mouse_to_touch: bool,

    /// True to exit after the page load (`-x`).
    pub exit_after_load: bool,

    /// Do not use native titlebar
    pub no_native_titlebar: bool,

    /// Enable vsync in the compositor
    pub enable_vsync: bool,

    /// True to show webrender profiling stats on screen.
    pub webrender_stats: bool,

    /// True if webrender recording should be enabled.
    pub webrender_record: bool,

    /// True if webrender is allowed to batch draw calls as instances.
    pub webrender_batch: bool,

    /// Load shaders from disk.
    pub shaders_dir: Option<PathBuf>,

    /// True to compile all webrender shaders at init time. This is mostly
    /// useful when modifying the shaders, to ensure they all compile
    /// after each change is made.
    pub precache_shaders: bool,

    /// True if WebRender should use multisample antialiasing.
    pub use_msaa: bool,

    /// Directory for a default config directory
    pub config_dir: Option<PathBuf>,

    // don't skip any backtraces on panic
    pub full_backtraces: bool,

    /// True to use OS native signposting facilities. This makes profiling events (script activity,
    /// reflow, compositing, etc.) appear in Instruments.app on macOS.
    pub signpost: bool,

    /// Print the version and exit.
    pub is_printing_version: bool,

    /// Path to SSL certificates.
    pub certificate_path: Option<String>,

    /// Unminify Javascript.
    pub unminify_js: bool,

    /// Print Progressive Web Metrics to console.
    pub print_pwm: bool,

    /// Only shutdown once all theads are finished.
    pub clean_shutdown: bool,
}

fn print_usage(app: &str, opts: &Options) {
    let message = format!(
        "Usage: {} [ options ... ] [URL]\n\twhere options include",
        app
    );
    println!("{}", opts.usage(&message));
}

/// Debug options for Servo, currently set on the command line with -Z
#[derive(Default)]
pub struct DebugOptions {
    /// List all the debug options.
    pub help: bool,

    /// Bubble intrinsic widths separately like other engines.
    pub bubble_widths: bool,

    /// Disable antialiasing of rendered text.
    pub disable_text_aa: bool,

    /// Disable subpixel antialiasing of rendered text.
    pub disable_subpixel_aa: bool,

    /// Disable antialiasing of rendered text on the HTML canvas element.
    pub disable_canvas_aa: bool,

    /// Print the DOM after each restyle.
    pub dump_style_tree: bool,

    /// Dumps the rule tree.
    pub dump_rule_tree: bool,

    /// Print the flow tree after each layout.
    pub dump_flow_tree: bool,

    /// Print the display list after each layout.
    pub dump_display_list: bool,

    /// Print the display list in JSON form.
    pub dump_display_list_json: bool,

    /// Print notifications when there is a relayout.
    pub relayout_event: bool,

    /// Profile which events script threads spend their time on.
    pub profile_script_events: bool,

    /// Enable all heartbeats for profiling.
    pub profile_heartbeats: bool,

    /// Paint borders along fragment boundaries.
    pub show_fragment_borders: bool,

    /// Mark which thread laid each flow out with colors.
    pub show_parallel_layout: bool,

    /// Write layout trace to an external file for debugging.
    pub trace_layout: bool,

    /// Disable the style sharing cache.
    pub disable_share_style_cache: bool,

    /// Whether to show in stdout style sharing cache stats after a restyle.
    pub style_sharing_stats: bool,

    /// Translate mouse input into touch events.
    pub convert_mouse_to_touch: bool,

    /// Replace unpaires surrogates in DOM strings with U+FFFD.
    /// See <https://github.com/servo/servo/issues/6564>
    pub replace_surrogates: bool,

    /// Log GC passes and their durations.
    pub gc_profile: bool,

    /// Load web fonts synchronously to avoid non-deterministic network-driven reflows.
    pub load_webfonts_synchronously: bool,

    /// Disable vsync in the compositor
    pub disable_vsync: bool,

    /// Show webrender profiling stats on screen.
    pub webrender_stats: bool,

    /// Enable webrender recording.
    pub webrender_record: bool,

    /// Enable webrender instanced draw call batching.
    pub webrender_disable_batch: bool,

    /// Use multisample antialiasing in WebRender.
    pub use_msaa: bool,

    // don't skip any backtraces on panic
    pub full_backtraces: bool,

    /// True to compile all webrender shaders at init time. This is mostly
    /// useful when modifying the shaders, to ensure they all compile
    /// after each change is made.
    pub precache_shaders: bool,

    /// True to use OS native signposting facilities. This makes profiling events (script activity,
    /// reflow, compositing, etc.) appear in Instruments.app on macOS.
    pub signpost: bool,
}

impl DebugOptions {
    pub fn extend(&mut self, debug_string: String) -> Result<(), String> {
        for option in debug_string.split(',') {
            match option {
                "help" => self.help = true,
                "bubble-widths" => self.bubble_widths = true,
                "disable-text-aa" => self.disable_text_aa = true,
                "disable-subpixel-aa" => self.disable_subpixel_aa = true,
                "disable-canvas-aa" => self.disable_text_aa = true,
                "dump-style-tree" => self.dump_style_tree = true,
                "dump-rule-tree" => self.dump_rule_tree = true,
                "dump-flow-tree" => self.dump_flow_tree = true,
                "dump-display-list" => self.dump_display_list = true,
                "dump-display-list-json" => self.dump_display_list_json = true,
                "relayout-event" => self.relayout_event = true,
                "profile-script-events" => self.profile_script_events = true,
                "profile-heartbeats" => self.profile_heartbeats = true,
                "show-fragment-borders" => self.show_fragment_borders = true,
                "show-parallel-layout" => self.show_parallel_layout = true,
                "trace-layout" => self.trace_layout = true,
                "disable-share-style-cache" => self.disable_share_style_cache = true,
                "style-sharing-stats" => self.style_sharing_stats = true,
                "convert-mouse-to-touch" => self.convert_mouse_to_touch = true,
                "replace-surrogates" => self.replace_surrogates = true,
                "gc-profile" => self.gc_profile = true,
                "load-webfonts-synchronously" => self.load_webfonts_synchronously = true,
                "disable-vsync" => self.disable_vsync = true,
                "wr-stats" => self.webrender_stats = true,
                "wr-record" => self.webrender_record = true,
                "wr-no-batch" => self.webrender_disable_batch = true,
                "msaa" => self.use_msaa = true,
                "full-backtraces" => self.full_backtraces = true,
                "precache-shaders" => self.precache_shaders = true,
                "signpost" => self.signpost = true,
                "" => {},
                _ => return Err(String::from(option)),
            };
        }
        Ok(())
    }
}

fn print_debug_usage(app: &str) -> ! {
    fn print_option(name: &str, description: &str) {
        println!("\t{:<35} {}", name, description);
    }

    println!(
        "Usage: {} debug option,[options,...]\n\twhere options include\n\nOptions:",
        app
    );

    print_option(
        "bubble-widths",
        "Bubble intrinsic widths separately like other engines.",
    );
    print_option("disable-text-aa", "Disable antialiasing of rendered text.");
    print_option(
        "disable-canvas-aa",
        "Disable antialiasing on the HTML canvas element.",
    );
    print_option(
        "dump-style-tree",
        "Print the DOM with computed styles after each restyle.",
    );
    print_option("dump-flow-tree", "Print the flow tree after each layout.");
    print_option(
        "dump-display-list",
        "Print the display list after each layout.",
    );
    print_option(
        "dump-display-list-json",
        "Print the display list in JSON form.",
    );
    print_option(
        "relayout-event",
        "Print notifications when there is a relayout.",
    );
    print_option(
        "profile-script-events",
        "Enable profiling of script-related events.",
    );
    print_option(
        "profile-heartbeats",
        "Enable heartbeats for all thread categories.",
    );
    print_option(
        "show-fragment-borders",
        "Paint borders along fragment boundaries.",
    );
    print_option(
        "show-parallel-layout",
        "Mark which thread laid each flow out with colors.",
    );
    print_option(
        "trace-layout",
        "Write layout trace to an external file for debugging.",
    );
    print_option(
        "disable-share-style-cache",
        "Disable the style sharing cache.",
    );
    print_option(
        "parallel-display-list-building",
        "Build display lists in parallel.",
    );
    print_option(
        "convert-mouse-to-touch",
        "Send touch events instead of mouse events",
    );
    print_option(
        "replace-surrogates",
        "Replace unpaires surrogates in DOM strings with U+FFFD. \
         See https://github.com/servo/servo/issues/6564",
    );
    print_option("gc-profile", "Log GC passes and their durations.");
    print_option(
        "load-webfonts-synchronously",
        "Load web fonts synchronously to avoid non-deterministic network-driven reflows",
    );
    print_option(
        "disable-vsync",
        "Disable vsync mode in the compositor to allow profiling at more than monitor refresh rate",
    );
    print_option("wr-stats", "Show WebRender profiler on screen.");
    print_option("msaa", "Use multisample antialiasing in WebRender.");
    print_option("full-backtraces", "Print full backtraces for all errors");
    print_option("wr-debug", "Display webrender tile borders.");
    print_option("wr-no-batch", "Disable webrender instanced batching.");
    print_option("precache-shaders", "Compile all shaders during init.");
    print_option(
        "signpost",
        "Emit native OS signposts for profile events (currently macOS only)",
    );

    println!("");

    process::exit(0)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum OutputOptions {
    /// Database connection config (hostname, name, user, pass)
    DB(ServoUrl, Option<String>, Option<String>, Option<String>),
    FileName(String),
    Stdout(f64),
}

fn args_fail(msg: &str) -> ! {
    writeln!(io::stderr(), "{}", msg).unwrap();
    process::exit(1)
}

static MULTIPROCESS: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn multiprocess() -> bool {
    MULTIPROCESS.load(Ordering::Relaxed)
}

enum UserAgent {
    Desktop,
    Android,
    #[allow(non_camel_case_types)]
    iOS,
}

fn default_user_agent_string(agent: UserAgent) -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    const DESKTOP_UA_STRING: &'static str =
        "Mozilla/5.0 (X11; Linux x86_64; rv:63.0) Servo/1.0 Firefox/63.0";
    #[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
    const DESKTOP_UA_STRING: &'static str =
        "Mozilla/5.0 (X11; Linux i686; rv:63.0) Servo/1.0 Firefox/63.0";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    const DESKTOP_UA_STRING: &'static str =
        "Mozilla/5.0 (Windows NT 6.1; Win64; x64; rv:63.0) Servo/1.0 Firefox/63.0";
    #[cfg(all(target_os = "windows", not(target_arch = "x86_64")))]
    const DESKTOP_UA_STRING: &'static str =
        "Mozilla/5.0 (Windows NT 6.1; rv:63.0) Servo/1.0 Firefox/63.0";

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    // Neither Linux nor Windows, so maybe OS X, and if not then OS X is an okay fallback.
    const DESKTOP_UA_STRING: &'static str =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.10; rv:63.0) Servo/1.0 Firefox/63.0";

    match agent {
        UserAgent::Desktop => DESKTOP_UA_STRING,
        UserAgent::Android => "Mozilla/5.0 (Android; Mobile; rv:63.0) Servo/1.0 Firefox/63.0",
        UserAgent::iOS => {
            "Mozilla/5.0 (iPhone; CPU iPhone OS 8_3 like Mac OS X; rv:63.0) Servo/1.0 Firefox/63.0"
        },
    }
}

#[cfg(target_os = "android")]
const DEFAULT_USER_AGENT: UserAgent = UserAgent::Android;

#[cfg(target_os = "ios")]
const DEFAULT_USER_AGENT: UserAgent = UserAgent::iOS;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
const DEFAULT_USER_AGENT: UserAgent = UserAgent::Desktop;

pub fn default_opts() -> Opts {
    Opts {
        is_running_problem_test: false,
        url: None,
        tile_size: 512,
        device_pixels_per_px: None,
        time_profiling: None,
        time_profiler_trace_path: None,
        mem_profiler_period: None,
        nonincremental_layout: false,
        userscripts: None,
        user_stylesheets: Vec::new(),
        output_file: None,
        replace_surrogates: false,
        gc_profile: false,
        load_webfonts_synchronously: false,
        headless: false,
        angle: false,
        hard_fail: true,
        bubble_inline_sizes_separately: false,
        show_debug_fragment_borders: false,
        show_debug_parallel_layout: false,
        enable_text_antialiasing: true,
        enable_subpixel_text_antialiasing: true,
        enable_canvas_antialiasing: true,
        trace_layout: false,
        debugger_port: None,
        devtools_port: None,
        webdriver_port: None,
        initial_window_size: TypedSize2D::new(1024, 740),
        user_agent: default_user_agent_string(DEFAULT_USER_AGENT).into(),
        multiprocess: false,
        random_pipeline_closure_probability: None,
        random_pipeline_closure_seed: None,
        sandbox: false,
        dump_style_tree: false,
        dump_rule_tree: false,
        dump_flow_tree: false,
        dump_display_list: false,
        dump_display_list_json: false,
        relayout_event: false,
        profile_script_events: false,
        profile_heartbeats: false,
        disable_share_style_cache: false,
        style_sharing_stats: false,
        convert_mouse_to_touch: false,
        exit_after_load: false,
        no_native_titlebar: false,
        enable_vsync: true,
        webrender_stats: false,
        use_msaa: false,
        config_dir: None,
        full_backtraces: false,
        is_printing_version: false,
        webrender_record: false,
        webrender_batch: true,
        shaders_dir: None,
        precache_shaders: false,
        signpost: false,
        certificate_path: None,
        unminify_js: false,
        print_pwm: false,
        clean_shutdown: false,
    }
}

pub fn from_cmdline_args(args: &[String]) -> ArgumentParsingResult {
    let (app_name, args) = args.split_first().unwrap();

    let mut opts = Options::new();
    opts.optflag("c", "cpu", "CPU painting");
    opts.optflag("g", "gpu", "GPU painting");
    opts.optopt("o", "output", "Output file", "output.png");
    opts.optopt("s", "size", "Size of tiles", "512");
    opts.optopt("", "device-pixel-ratio", "Device pixels per px", "");
    opts.optflagopt(
        "p",
        "profile",
        "Time profiler flag and either a TSV output filename \
         OR an interval for output to Stdout (blank for Stdout with interval of 5s)",
        "10 \
         OR time.tsv",
    );
    opts.optflagopt(
        "",
        "profiler-trace-path",
        "Path to dump a self-contained HTML timeline of profiler traces",
        "",
    );
    opts.optflagopt(
        "m",
        "memory-profile",
        "Memory profiler flag and output interval",
        "10",
    );
    opts.optflag("x", "exit", "Exit after load flag");
    opts.optopt(
        "y",
        "layout-threads",
        "Number of threads to use for layout",
        "1",
    );
    opts.optflag(
        "i",
        "nonincremental-layout",
        "Enable to turn off incremental layout.",
    );
    opts.optflagopt(
        "",
        "userscripts",
        "Uses userscripts in resources/user-agent-js, or a specified full path",
        "",
    );
    opts.optmulti(
        "",
        "user-stylesheet",
        "A user stylesheet to be added to every document",
        "file.css",
    );
    opts.optopt(
        "",
        "shaders",
        "Shaders will be loaded from the specified directory instead of using the builtin ones.",
        "",
    );
    opts.optflag("z", "headless", "Headless mode");
    opts.optflag(
        "",
        "angle",
        "Use ANGLE to create a GL context (Windows-only)",
    );
    opts.optflag(
        "f",
        "hard-fail",
        "Exit on thread failure instead of displaying about:failure",
    );
    opts.optflag(
        "F",
        "soft-fail",
        "Display about:failure on thread failure instead of exiting",
    );
    opts.optflagopt(
        "",
        "remote-debugging-port",
        "Start remote debugger server on port",
        "2794",
    );
    opts.optflagopt(
        "",
        "devtools",
        "Start remote devtools server on port",
        "6000",
    );
    opts.optflagopt(
        "",
        "webdriver",
        "Start remote WebDriver server on port",
        "7000",
    );
    opts.optopt("", "resolution", "Set window resolution.", "1024x740");
    opts.optopt(
        "u",
        "user-agent",
        "Set custom user agent string (or ios / android / desktop for platform default)",
        "NCSA Mosaic/1.0 (X11;SunOS 4.1.4 sun4m)",
    );
    opts.optflag("M", "multiprocess", "Run in multiprocess mode");
    opts.optflag("S", "sandbox", "Run in a sandbox if multiprocess");
    opts.optopt(
        "",
        "random-pipeline-closure-probability",
        "Probability of randomly closing a pipeline (for testing constellation hardening).",
        "0.0",
    );
    opts.optopt(
        "",
        "random-pipeline-closure-seed",
        "A fixed seed for repeatbility of random pipeline closure.",
        "",
    );
    opts.optmulti(
        "Z",
        "debug",
        "A comma-separated string of debug options. Pass help to show available options.",
        "",
    );
    opts.optflag("h", "help", "Print this message");
    opts.optopt(
        "",
        "resources-path",
        "Path to find static resources",
        "/home/servo/resources",
    );
    opts.optopt(
        "",
        "certificate-path",
        "Path to find SSL certificates",
        "/home/servo/resources/certs",
    );
    opts.optopt(
        "",
        "content-process",
        "Run as a content process and connect to the given pipe",
        "servo-ipc-channel.abcdefg",
    );
    opts.optmulti(
        "",
        "pref",
        "A preference to set to enable",
        "dom.bluetooth.enabled",
    );
    opts.optflag("b", "no-native-titlebar", "Do not use native titlebar");
    opts.optflag("w", "webrender", "Use webrender backend");
    opts.optopt("G", "graphics", "Select graphics backend (gl or es2)", "gl");
    opts.optopt(
        "",
        "config-dir",
        "config directory following xdg spec on linux platform",
        "",
    );
    opts.optflag(
        "",
        "clean-shutdown",
        "Do not shutdown until all threads have finished (macos only)",
    );
    opts.optflag("v", "version", "Display servo version information");
    opts.optflag("", "unminify-js", "Unminify Javascript");
    opts.optopt("", "profiler-db-user", "Profiler database user", "");
    opts.optopt("", "profiler-db-pass", "Profiler database password", "");
    opts.optopt("", "profiler-db-name", "Profiler database name", "");
    opts.optflag("", "print-pwm", "Print Progressive Web Metrics");

    let opt_match = match opts.parse(args) {
        Ok(m) => m,
        Err(f) => args_fail(&f.to_string()),
    };

    if opt_match.opt_present("h") || opt_match.opt_present("help") {
        print_usage(app_name, &opts);
        process::exit(0);
    };

    // If this is the content process, we'll receive the real options over IPC. So just fill in
    // some dummy options for now.
    if let Some(content_process) = opt_match.opt_str("content-process") {
        MULTIPROCESS.store(true, Ordering::SeqCst);
        return ArgumentParsingResult::ContentProcess(content_process);
    }

    let mut debug_options = DebugOptions::default();

    for debug_string in opt_match.opt_strs("Z") {
        if let Err(e) = debug_options.extend(debug_string) {
            args_fail(&format!("error: unrecognized debug option: {}", e));
        }
    }

    if debug_options.help {
        print_debug_usage(app_name)
    }

    let cwd = env::current_dir().unwrap();
    let url_opt = if !opt_match.free.is_empty() {
        Some(&opt_match.free[0][..])
    } else {
        None
    };
    let is_running_problem_test = url_opt.as_ref().map_or(false, |url| {
        url.starts_with("http://web-platform.test:8000/2dcontext/drawing-images-to-the-canvas/") ||
            url.starts_with("http://web-platform.test:8000/_mozilla/mozilla/canvas/") ||
            url.starts_with("http://web-platform.test:8000/_mozilla/css/canvas_over_area.html")
    });

    let url_opt = url_opt.and_then(|url_string| {
        parse_url_or_filename(&cwd, url_string)
            .or_else(|error| {
                warn!("URL parsing failed ({:?}).", error);
                Err(error)
            })
            .ok()
    });

    let tile_size: usize = match opt_match.opt_str("s") {
        Some(tile_size_str) => tile_size_str
            .parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: -s ({})", err))),
        None => 512,
    };

    let device_pixels_per_px = opt_match.opt_str("device-pixel-ratio").map(|dppx_str| {
        dppx_str.parse().unwrap_or_else(|err| {
            args_fail(&format!(
                "Error parsing option: --device-pixel-ratio ({})",
                err
            ))
        })
    });

    // If only the flag is present, default to a 5 second period for both profilers
    let time_profiling = if opt_match.opt_present("p") {
        match opt_match.opt_str("p") {
            Some(argument) => match argument.parse::<f64>() {
                Ok(interval) => Some(OutputOptions::Stdout(interval)),
                Err(_) => match ServoUrl::parse(&argument) {
                    Ok(url) => Some(OutputOptions::DB(
                        url,
                        opt_match.opt_str("profiler-db-name"),
                        opt_match.opt_str("profiler-db-user"),
                        opt_match.opt_str("profiler-db-pass"),
                    )),
                    Err(_) => Some(OutputOptions::FileName(argument)),
                },
            },
            None => Some(OutputOptions::Stdout(5.0 as f64)),
        }
    } else {
        // if the p option doesn't exist:
        None
    };

    if let Some(ref time_profiler_trace_path) = opt_match.opt_str("profiler-trace-path") {
        let mut path = PathBuf::from(time_profiler_trace_path);
        path.pop();
        if let Err(why) = fs::create_dir_all(&path) {
            error!(
                "Couldn't create/open {:?}: {:?}",
                Path::new(time_profiler_trace_path).to_string_lossy(),
                why
            );
        }
    }

    let mem_profiler_period = opt_match.opt_default("m", "5").map(|period| {
        period
            .parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: -m ({})", err)))
    });

    let mut layout_threads: Option<usize> = opt_match.opt_str("y").map(|layout_threads_str| {
        layout_threads_str
            .parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: -y ({})", err)))
    });

    let nonincremental_layout = opt_match.opt_present("i");

    let random_pipeline_closure_probability = opt_match
        .opt_str("random-pipeline-closure-probability")
        .map(|prob| {
            prob.parse().unwrap_or_else(|err| {
                args_fail(&format!(
                    "Error parsing option: --random-pipeline-closure-probability ({})",
                    err
                ))
            })
        });

    let random_pipeline_closure_seed =
        opt_match
            .opt_str("random-pipeline-closure-seed")
            .map(|seed| {
                seed.parse().unwrap_or_else(|err| {
                    args_fail(&format!(
                        "Error parsing option: --random-pipeline-closure-seed ({})",
                        err
                    ))
                })
            });

    let mut bubble_inline_sizes_separately = debug_options.bubble_widths;
    if debug_options.trace_layout {
        layout_threads = Some(1);
        bubble_inline_sizes_separately = true;
    }

    let debugger_port = opt_match
        .opt_default("remote-debugging-port", "2794")
        .map(|port| {
            port.parse().unwrap_or_else(|err| {
                args_fail(&format!(
                    "Error parsing option: --remote-debugging-port ({})",
                    err
                ))
            })
        });

    let devtools_port = opt_match.opt_default("devtools", "6000").map(|port| {
        port.parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: --devtools ({})", err)))
    });

    let webdriver_port = opt_match.opt_default("webdriver", "7000").map(|port| {
        port.parse().unwrap_or_else(|err| {
            args_fail(&format!("Error parsing option: --webdriver ({})", err))
        })
    });

    let initial_window_size = match opt_match.opt_str("resolution") {
        Some(res_string) => {
            let res: Vec<u32> = res_string
                .split('x')
                .map(|r| {
                    r.parse().unwrap_or_else(|err| {
                        args_fail(&format!("Error parsing option: --resolution ({})", err))
                    })
                })
                .collect();
            TypedSize2D::new(res[0], res[1])
        },
        None => TypedSize2D::new(1024, 740),
    };

    if opt_match.opt_present("M") {
        MULTIPROCESS.store(true, Ordering::SeqCst)
    }

    let user_agent = match opt_match.opt_str("u") {
        Some(ref ua) if ua == "ios" => default_user_agent_string(UserAgent::iOS).into(),
        Some(ref ua) if ua == "android" => default_user_agent_string(UserAgent::Android).into(),
        Some(ref ua) if ua == "desktop" => default_user_agent_string(UserAgent::Desktop).into(),
        Some(ua) => ua.into(),
        None => default_user_agent_string(DEFAULT_USER_AGENT).into(),
    };

    let user_stylesheets = opt_match
        .opt_strs("user-stylesheet")
        .iter()
        .map(|filename| {
            let path = cwd.join(filename);
            let url = ServoUrl::from_url(Url::from_file_path(&path).unwrap());
            let mut contents = Vec::new();
            File::open(path)
                .unwrap_or_else(|err| args_fail(&format!("Couldn't open {}: {}", filename, err)))
                .read_to_end(&mut contents)
                .unwrap_or_else(|err| args_fail(&format!("Couldn't read {}: {}", filename, err)));
            (contents, url)
        })
        .collect();

    let do_not_use_native_titlebar =
        opt_match.opt_present("b") || !(pref!(shell.native_titlebar.enabled));

    let enable_subpixel_text_antialiasing =
        !debug_options.disable_subpixel_aa && pref!(gfx.subpixel_text_antialiasing.enabled);

    let is_printing_version = opt_match.opt_present("v") || opt_match.opt_present("version");

    let opts = Opts {
        is_running_problem_test: is_running_problem_test,
        url: url_opt,
        tile_size: tile_size,
        device_pixels_per_px: device_pixels_per_px,
        time_profiling: time_profiling,
        time_profiler_trace_path: opt_match.opt_str("profiler-trace-path"),
        mem_profiler_period: mem_profiler_period,
        nonincremental_layout: nonincremental_layout,
        userscripts: opt_match.opt_default("userscripts", ""),
        user_stylesheets: user_stylesheets,
        output_file: opt_match.opt_str("o"),
        replace_surrogates: debug_options.replace_surrogates,
        gc_profile: debug_options.gc_profile,
        load_webfonts_synchronously: debug_options.load_webfonts_synchronously,
        headless: opt_match.opt_present("z"),
        angle: opt_match.opt_present("angle"),
        hard_fail: opt_match.opt_present("f") && !opt_match.opt_present("F"),
        bubble_inline_sizes_separately: bubble_inline_sizes_separately,
        profile_script_events: debug_options.profile_script_events,
        profile_heartbeats: debug_options.profile_heartbeats,
        trace_layout: debug_options.trace_layout,
        debugger_port: debugger_port,
        devtools_port: devtools_port,
        webdriver_port: webdriver_port,
        initial_window_size: initial_window_size,
        user_agent: user_agent,
        multiprocess: opt_match.opt_present("M"),
        sandbox: opt_match.opt_present("S"),
        random_pipeline_closure_probability: random_pipeline_closure_probability,
        random_pipeline_closure_seed: random_pipeline_closure_seed,
        show_debug_fragment_borders: debug_options.show_fragment_borders,
        show_debug_parallel_layout: debug_options.show_parallel_layout,
        enable_text_antialiasing: !debug_options.disable_text_aa,
        enable_subpixel_text_antialiasing: enable_subpixel_text_antialiasing,
        enable_canvas_antialiasing: !debug_options.disable_canvas_aa,
        dump_style_tree: debug_options.dump_style_tree,
        dump_rule_tree: debug_options.dump_rule_tree,
        dump_flow_tree: debug_options.dump_flow_tree,
        dump_display_list: debug_options.dump_display_list,
        dump_display_list_json: debug_options.dump_display_list_json,
        relayout_event: debug_options.relayout_event,
        disable_share_style_cache: debug_options.disable_share_style_cache,
        style_sharing_stats: debug_options.style_sharing_stats,
        convert_mouse_to_touch: debug_options.convert_mouse_to_touch,
        exit_after_load: opt_match.opt_present("x"),
        no_native_titlebar: do_not_use_native_titlebar,
        enable_vsync: !debug_options.disable_vsync,
        webrender_stats: debug_options.webrender_stats,
        use_msaa: debug_options.use_msaa,
        config_dir: opt_match.opt_str("config-dir").map(Into::into),
        full_backtraces: debug_options.full_backtraces,
        is_printing_version: is_printing_version,
        webrender_record: debug_options.webrender_record,
        webrender_batch: !debug_options.webrender_disable_batch,
        shaders_dir: opt_match.opt_str("shaders").map(Into::into),
        precache_shaders: debug_options.precache_shaders,
        signpost: debug_options.signpost,
        certificate_path: opt_match.opt_str("certificate-path"),
        unminify_js: opt_match.opt_present("unminify-js"),
        print_pwm: opt_match.opt_present("print-pwm"),
        clean_shutdown: opt_match.opt_present("clean-shutdown"),
    };

    set_options(opts);

    // These must happen after setting the default options, since the prefs rely on
    // on the resource path.
    // Note that command line preferences have the highest precedence

    prefs::add_user_prefs();

    for pref in opt_match.opt_strs("pref").iter() {
        parse_pref_from_command_line(pref);
    }

    if let Some(layout_threads) = layout_threads {
        set_pref!(layout.threads, layout_threads as i64);
    }

    ArgumentParsingResult::ChromeProcess
}

pub enum ArgumentParsingResult {
    ChromeProcess,
    ContentProcess(String),
}

// Make Opts available globally. This saves having to clone and pass
// opts everywhere it is used, which gets particularly cumbersome
// when passing through the DOM structures.
lazy_static! {
    static ref OPTIONS: RwLock<Opts> = RwLock::new(default_opts());
}

pub fn set_options(opts: Opts) {
    MULTIPROCESS.store(opts.multiprocess, Ordering::SeqCst);
    *OPTIONS.write().unwrap() = opts;
}

#[inline]
pub fn get() -> RwLockReadGuard<'static, Opts> {
    OPTIONS.read().unwrap()
}

pub fn parse_pref_from_command_line(pref: &str) {
    let split: Vec<&str> = pref.splitn(2, '=').collect();
    let pref_name = split[0];
    let pref_value = parse_cli_pref_value(split.get(1).cloned());
    prefs::pref_map()
        .set(pref_name, pref_value)
        .expect(format!("Error setting preference: {}", pref).as_str());
}

fn parse_cli_pref_value(input: Option<&str>) -> PrefValue {
    match input {
        Some("true") | None => PrefValue::Bool(true),
        Some("false") => PrefValue::Bool(false),
        Some(string) => {
            if let Some(int) = string.parse::<i64>().ok() {
                PrefValue::Int(int)
            } else if let Some(float) = string.parse::<f64>().ok() {
                PrefValue::Float(float)
            } else {
                PrefValue::from(string)
            }
        },
    }
}

pub fn parse_url_or_filename(cwd: &Path, input: &str) -> Result<ServoUrl, ()> {
    match ServoUrl::parse(input) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Url::from_file_path(&*cwd.join(input)).map(ServoUrl::from_url)
        },
        Err(_) => Err(()),
    }
}

impl Opts {
    pub fn should_use_osmesa(&self) -> bool {
        self.headless
    }
}
