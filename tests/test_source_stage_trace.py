import unittest
from scripts.source_stage_trace import StageTrace


class StageTraceTests(unittest.TestCase):
    def test_only_known_client_markers_not_arbitrary_names(self):
        trace = StageTrace()
        trace.feed(b"[youtube] private-id: Downloading tv samsung player API JSON\n")
        trace.feed(b"[youtube] private-id: Downloading secret attacker player API JSON\n")
        trace.feed(b"Cookie: private-session\nhttps://signed.example/?secret=1\n")
        self.assertEqual(trace.report()['client_request_markers'], ['tv_samsung'])
        self.assertNotIn('private', str(trace.report()))
        self.assertNotIn('secret', str(trace.report()))

    def test_configured_node_is_not_observed_solver_execution(self):
        trace = StageTrace()
        trace.feed(b"[debug] JS runtimes: node-22.22.1\n")
        self.assertEqual(trace.report()['js_provider_markers'], [])
        self.assertFalse(trace.report()['js_challenge_marker_seen'])
        trace.feed(b"[youtube] [jsc:node] Solving JS challenges using node\n")
        self.assertEqual(trace.report()['js_provider_markers'], ['node'])
        self.assertTrue(trace.report()['js_challenge_marker_seen'])

    def test_chunk_boundaries_and_webpage_marker(self):
        trace = StageTrace()
        trace.feed(b"[youtube] id: Downloading web")
        trace.feed(b"page\n[youtube] id: Downloading vis")
        trace.feed(b"ionos player API JSON\r\n")
        self.assertTrue(trace.report()['webpage_request_marker_seen'])
        self.assertEqual(trace.report()['client_request_markers'], ['visionos'])

    def test_long_lines_and_total_limit_are_lossy_and_bounded(self):
        trace = StageTrace()
        trace.feed(b"x" * 1000000)
        trace.feed(b"\n[youtube] id: Downloading web player API JSON\n")
        self.assertLessEqual(len(trace._pending), trace.MAX_LINE)
        self.assertTrue(trace.report()['classification_truncated'])
        self.assertEqual(trace.report()['client_request_markers'], [])

    def test_oversize_line_does_not_inject_a_suffix_marker(self):
        trace = StageTrace()
        trace.feed(b"x" * (trace.MAX_LINE + 1))
        trace.feed(b"[youtube] id: Downloading web player API JSON\n")
        trace.feed(b"[youtube] id: Downloading ios player API JSON\n")
        self.assertEqual(trace.report()['client_request_markers'], ['ios'])

    def test_unknown_runtime_and_unterminated_line_are_not_reported(self):
        trace = StageTrace()
        trace.feed(b"[jsc:secret-engine] Solving JS challenges\n")
        trace.feed(b"[youtube] id: Downloading web player API JSON")
        self.assertEqual(trace.report()['js_provider_markers'], [])
        self.assertEqual(trace.report()['client_request_markers'], [])


if __name__ == '__main__':
    unittest.main()
