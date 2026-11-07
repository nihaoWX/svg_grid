// --------------------------------------------------------------------------- #
// W9: a `url(#…)` reference whose target nothing defines (round-data/R16).
//
// The defect this warns about was paint servers (`<linearGradient>` /
// `<radialGradient>` / `<pattern>`) dropped by the converter while the
// `fill="url(#…)"` references were kept: nothing failed, no element moved, the
// shapes were simply painted with nothing — whole series missing from a figure
// that still "looked complete". W9 is the mechanical check that keeps that
// silent ink loss from shipping again (a dropped `<clipPath>` is the same class).
//
// The warning is one line per call, answers *what / why / what to do*, never
// changes the exit code or the output bytes, and prints nothing at all when
// every reference resolves.
// --------------------------------------------------------------------------- #

use std::fs;

use crate::common::*;

#[test]
fn w9_fires_for_a_dangling_url_and_is_silent_when_every_id_resolves() {
    // W9 (round-data/R16): a `url(#…)` whose target nothing defines is *not* fatal
    // and does not move anything — the shape is simply painted with nothing, which
    // is exactly why it needs to be said out loud. The clean twin must be silent.
    let dangling_panel = temp_path("dangling_url.svg");
    fs::write(
        &dangling_panel,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#no-such-id)"/></g>
</svg>"##,
    )
    .unwrap();
    let clean_panel = temp_path("dangling_url_clean.svg");
    fs::write(
        &clean_panel,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="10"><stop offset="0" stop-color="#fff"/></linearGradient></defs>
<rect x="10" y="10" width="40" height="30" fill="url(#g)"/></g>
</svg>"##,
    )
    .unwrap();

    let (fired, fired_out) = run_cli(
        "dangling_url_out.svg",
        std::slice::from_ref(&dangling_panel),
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(fired.status.success(), "W9 must not fail the run");
    assert!(
        fired_out.is_file(),
        "W9 must not change the output contract: the SVG is still written"
    );
    let stderr = String::from_utf8_lossy(&fired.stderr);
    // What happened: how many references, which input/element/attribute, which id.
    assert!(
        stderr.contains("1 reference(s) point at an id that nothing in the output defines"),
        "W9 must say what happened and how many: {stderr}"
    );
    assert!(
        stderr.contains("input #0") && stderr.contains("<rect fill>"),
        "W9 must name the input, element and attribute: {stderr}"
    );
    assert!(
        stderr.contains("url(#no-such-id)"),
        "W9 must name the dangling id: {stderr}"
    );
    // Why it matters: the shape is painted with nothing, so the figure loses ink silently.
    assert!(
        stderr.contains("painted with nothing") && stderr.contains("loses ink"),
        "W9 must explain the consequence (silent ink loss): {stderr}"
    );
    // What to do about it.
    assert!(stderr.contains("Fix:"), "W9 must carry a fix: {stderr}");
    assert!(
        stderr.contains("<defs>") && stderr.contains("<linearGradient>"),
        "W9's fix must name the <defs> paint server to keep: {stderr}"
    );
    assert!(
        stderr.contains("non-fatal warning(s)"),
        "W9 uses the shared tally: {stderr}"
    );

    let (quiet, quiet_out) = run_cli(
        "dangling_url_clean_out.svg",
        std::slice::from_ref(&clean_panel),
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(quiet.status.success());
    let quiet_stderr = String::from_utf8_lossy(&quiet.stderr);
    assert!(
        !quiet_stderr.contains("url(#"),
        "a resolved reference must be silent: {quiet_stderr}"
    );
    assert_eq!(
        quiet.stderr.len(),
        0,
        "a resolved reference must emit no W9 at all (and nothing else): {quiet_stderr}"
    );

    // Several dangling references in *one* input: one message, one count, every id named.
    // `clip-path` and an inline `style` count as separate references (not just `fill`).
    let many_panel = temp_path("dangling_url_many.svg");
    fs::write(
        &many_panel,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" clip-path="url(#no-such-clip)" style="fill: url(#no-such-fill)"/></g>
</svg>"##,
    )
    .unwrap();
    let (many, many_out) = run_cli(
        "dangling_url_many_out.svg",
        std::slice::from_ref(&many_panel),
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(many.status.success(), "W9 must not fail the run");
    let many_stderr = String::from_utf8_lossy(&many.stderr);
    let many_lines: Vec<&str> = many_stderr
        .lines()
        .filter(|line| line.contains("nothing in the output defines"))
        .collect();
    assert_eq!(
        many_lines.len(),
        1,
        "two dangling references in one input must be one W9 line: {many_stderr}"
    );
    assert!(
        many_lines[0].contains("2 reference(s)")
            && many_lines[0].contains("url(#no-such-clip)")
            && many_lines[0].contains("url(#no-such-fill)"),
        "W9 must count both references and name both ids: {many_stderr}"
    );

    for path in [
        fired_out,
        quiet_out,
        many_out,
        dangling_panel,
        clean_panel,
        many_panel,
    ] {
        let _ = fs::remove_file(path);
    }
}
