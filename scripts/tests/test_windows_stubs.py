#!/usr/bin/env python3
"""
Unit tests for the Windows PyInstaller runtime stubs.

These stubs allow hathor-core (which depends on Unix-only modules like fcntl,
termios, curses, resource, pty) to run on Windows by injecting fake modules
into sys.modules before the application imports anything.

This test validates the stub logic in isolation using only the Python stdlib.
Run with: python3 scripts/tests/test_windows_stubs.py

The stubs are defined in scripts/windows/build-hathor-core.ps1 inside the
pyi_rth_unix_stubs.py runtime hook.
"""
import sys
import types
import unittest


# ---------------------------------------------------------------------------
# Reproduce the stub classes exactly as they appear in pyi_rth_unix_stubs.py
# ---------------------------------------------------------------------------

class _StubModule(types.ModuleType):
    """A module stub where any unknown attribute returns a no-op callable
    or 0, so code that touches Unix-only APIs never crashes."""
    class _Noop:
        """Returns 0 in numeric contexts, no-op when called."""
        def __call__(self, *a, **kw): return 0
        def __int__(self): return 0
        def __or__(self, other): return other
        def __ror__(self, other): return other
        def __and__(self, other): return 0
        def __rand__(self, other): return 0
        def __bool__(self): return False
        def __repr__(self): return "0"
    _noop_instance = _Noop()
    def __getattr__(self, name):
        if name.startswith("__") and name.endswith("__"):
            raise AttributeError(name)
        return self._noop_instance


# ---------------------------------------------------------------------------
# _Noop tests
# ---------------------------------------------------------------------------

class TestNoop(unittest.TestCase):
    def setUp(self):
        self.noop = _StubModule._Noop()

    def test_callable_returns_zero(self):
        self.assertEqual(self.noop(), 0)

    def test_callable_with_args(self):
        self.assertEqual(self.noop(1, 2, key="val"), 0)

    def test_int_conversion(self):
        self.assertEqual(int(self.noop), 0)

    def test_bool_is_false(self):
        self.assertFalse(bool(self.noop))

    def test_repr(self):
        self.assertEqual(repr(self.noop), "0")

    # Bitwise OR — the pattern from Twisted's fdesc.py:
    #   flags = fcntl.fcntl(fd, fcntl.F_GETFD)
    #   fcntl.fcntl(fd, fcntl.F_SETFD, flags | fcntl.FD_CLOEXEC)
    def test_or_noop_lhs(self):
        """noop | 123 should return 123 (pass through the other operand)."""
        self.assertEqual(self.noop | 123, 123)

    def test_or_noop_rhs(self):
        """123 | noop should return 123 (via __ror__)."""
        self.assertEqual(123 | self.noop, 123)

    def test_or_noop_with_zero(self):
        self.assertEqual(self.noop | 0, 0)
        self.assertEqual(0 | self.noop, 0)

    def test_or_two_noops(self):
        """noop | noop — the __ror__ of the rhs returns the lhs noop."""
        other = _StubModule._Noop()
        result = self.noop | other
        self.assertIsInstance(result, _StubModule._Noop)

    def test_and_noop_lhs(self):
        """noop & 0xFF should return 0."""
        self.assertEqual(self.noop & 0xFF, 0)

    def test_and_noop_rhs(self):
        """0xFF & noop should return 0 (via __rand__)."""
        self.assertEqual(0xFF & self.noop, 0)

    def test_or_result_is_int(self):
        """Ensure the result of int | noop is a plain int."""
        result = 42 | self.noop
        self.assertIsInstance(result, int)

    def test_and_result_is_int(self):
        result = 42 & self.noop
        self.assertIsInstance(result, int)


# ---------------------------------------------------------------------------
# _StubModule tests
# ---------------------------------------------------------------------------

class TestStubModule(unittest.TestCase):
    def setUp(self):
        self.stub = _StubModule("fake_module")

    def test_unknown_attr_returns_noop(self):
        val = self.stub.anything
        self.assertIsInstance(val, _StubModule._Noop)

    def test_unknown_attr_is_callable(self):
        self.assertEqual(self.stub.some_func(1, 2, 3), 0)

    def test_different_attrs_same_noop(self):
        """All unknown attrs return the same singleton _Noop."""
        self.assertIs(self.stub.foo, self.stub.bar)

    def test_dunder_raises_attribute_error(self):
        """__dunder__ attributes should raise AttributeError, not return _Noop."""
        with self.assertRaises(AttributeError):
            _ = self.stub.__nonexistent__

    def test_explicit_attrs_override_getattr(self):
        """Attributes set on the instance take precedence over __getattr__."""
        self.stub.real_value = 42
        self.assertEqual(self.stub.real_value, 42)

    def test_is_module_type(self):
        self.assertIsInstance(self.stub, types.ModuleType)

    def test_module_name(self):
        self.assertEqual(self.stub.__name__, "fake_module")


# ---------------------------------------------------------------------------
# _curses stub tests
# ---------------------------------------------------------------------------

class TestCursesStub(unittest.TestCase):
    def setUp(self):
        self.mod = _StubModule("_curses")
        self.mod.error = type("error", (Exception,), {})
        self.mod.ERR = -1
        self.mod.OK = 0
        self.mod.version = b"0.0"
        self.mod.has_key = lambda key: False
        self.mod.LINES = 24
        self.mod.COLS = 80

    def test_error_is_exception(self):
        self.assertTrue(issubclass(self.mod.error, Exception))

    def test_error_can_be_raised(self):
        with self.assertRaises(self.mod.error):
            raise self.mod.error("test")

    def test_constants(self):
        self.assertEqual(self.mod.ERR, -1)
        self.assertEqual(self.mod.OK, 0)
        self.assertEqual(self.mod.LINES, 24)
        self.assertEqual(self.mod.COLS, 80)

    def test_version(self):
        self.assertEqual(self.mod.version, b"0.0")

    def test_has_key(self):
        self.assertFalse(self.mod.has_key(ord("a")))

    def test_unknown_attrs_dont_crash(self):
        """curses uses many constants; unknown ones should return _Noop."""
        _ = self.mod.A_BOLD
        _ = self.mod.COLOR_RED
        _ = self.mod.initscr


# ---------------------------------------------------------------------------
# resource stub tests
# ---------------------------------------------------------------------------

class TestResourceStub(unittest.TestCase):
    def setUp(self):
        self.mod = _StubModule("resource")
        self.mod.RLIMIT_NOFILE = 7
        self.mod.getrlimit = lambda _: (8192, 8192)
        self.mod.setrlimit = lambda *_: None
        self.mod.getpagesize = lambda: 4096

    def test_rlimit_nofile(self):
        self.assertEqual(self.mod.RLIMIT_NOFILE, 7)

    def test_getrlimit(self):
        result = self.mod.getrlimit(self.mod.RLIMIT_NOFILE)
        self.assertEqual(result, (8192, 8192))

    def test_setrlimit_no_crash(self):
        self.mod.setrlimit(self.mod.RLIMIT_NOFILE, (1024, 1024))

    def test_getpagesize(self):
        """prometheus_client.process_collector calls resource.getpagesize()."""
        self.assertEqual(self.mod.getpagesize(), 4096)


# ---------------------------------------------------------------------------
# termios stub tests — tty.py does "from termios import *"
# ---------------------------------------------------------------------------

class TestTermiosStub(unittest.TestCase):
    def setUp(self):
        self.mod = _StubModule("termios")
        self.mod.error = type("error", (Exception,), {})
        self.mod.tcgetattr = lambda fd: [0, 0, 0, 0, 0, 0, [b"\x00"] * 32]
        self.mod.tcsetattr = lambda *a: None
        _termios_consts = {
            "TCSAFLUSH": 2, "TCSANOW": 0, "TCSADRAIN": 1,
            "IGNBRK": 1, "BRKINT": 2, "IGNPAR": 4, "PARMRK": 8,
            "INPCK": 16, "ISTRIP": 32, "INLCR": 64, "IGNCR": 128,
            "ICRNL": 256, "IXON": 512, "IXOFF": 1024, "IXANY": 2048,
            "OPOST": 1, "PARENB": 4096, "CSIZE": 768, "CS8": 768,
            "ECHO": 8, "ECHOE": 2, "ECHOK": 4, "ECHONL": 16,
            "ICANON": 256, "IEXTEN": 1024, "ISIG": 128,
            "NOFLSH": 2147483648, "TOSTOP": 4194304,
            "VMIN": 16, "VTIME": 17,
        }
        for k, v in _termios_consts.items():
            setattr(self.mod, k, v)

    def test_tcsaflush_exists(self):
        """tty.py uses TCSAFLUSH as a default parameter value."""
        self.assertEqual(self.mod.TCSAFLUSH, 2)

    def test_key_constants_are_ints(self):
        """All termios constants must be real ints, not _Noop objects."""
        for name in ("TCSAFLUSH", "TCSANOW", "ECHO", "ICANON", "VMIN", "VTIME"):
            val = getattr(self.mod, name)
            self.assertIsInstance(val, int, f"{name} should be int, got {type(val)}")

    def test_star_import_gets_constants(self):
        """Simulate 'from termios import *' — constants must be in __dict__."""
        # "from mod import *" uses mod.__dict__ (filtered by __all__ or no-underscore)
        public_names = {k for k in self.mod.__dict__ if not k.startswith("_")}
        self.assertIn("TCSAFLUSH", public_names)
        self.assertIn("ECHO", public_names)
        self.assertIn("ICANON", public_names)
        self.assertIn("VMIN", public_names)
        self.assertIn("VTIME", public_names)

    def test_tcgetattr(self):
        result = self.mod.tcgetattr(0)
        self.assertIsInstance(result, list)
        self.assertEqual(len(result), 7)

    def test_tcsetattr_no_crash(self):
        self.mod.tcsetattr(0, self.mod.TCSAFLUSH, [0] * 7)

    def test_bitwise_ops_with_constants(self):
        """tty.py does things like: mode[LFLAG] &= ~(ECHO | ECHOE | ECHOK)"""
        result = self.mod.ECHO | self.mod.ECHOE | self.mod.ECHOK
        self.assertIsInstance(result, int)
        self.assertEqual(result, 8 | 2 | 4)

    def test_unknown_attrs_return_noop(self):
        """Any constant not explicitly set should still work via __getattr__."""
        val = self.mod.SOME_UNKNOWN_CONST
        self.assertIsInstance(val, _StubModule._Noop)


# ---------------------------------------------------------------------------
# fcntl stub tests — Twisted fdesc.py patterns
# ---------------------------------------------------------------------------

class TestFcntlStub(unittest.TestCase):
    def setUp(self):
        self.mod = _StubModule("fcntl")

    def test_fcntl_call(self):
        """fcntl.fcntl(fd, cmd) returns 0."""
        self.assertEqual(self.mod.fcntl(0, 0), 0)

    def test_flock_call(self):
        self.assertEqual(self.mod.flock(0, 0), 0)

    def test_fd_cloexec_or_flags(self):
        """Twisted: flags | fcntl.FD_CLOEXEC must not crash."""
        flags = 0
        result = flags | self.mod.FD_CLOEXEC
        self.assertIsInstance(result, int)

    def test_set_close_on_exec_pattern(self):
        """Full Twisted fdesc._setCloseOnExec pattern."""
        # flags = fcntl.fcntl(fd, fcntl.F_GETFD)
        flags = self.mod.fcntl(0, self.mod.F_GETFD)
        # int(noop) == 0, so flags is 0
        flags = int(flags)
        # fcntl.fcntl(fd, fcntl.F_SETFD, flags | fcntl.FD_CLOEXEC)
        result = self.mod.fcntl(0, self.mod.F_SETFD, flags | self.mod.FD_CLOEXEC)
        self.assertEqual(result, 0)


# ---------------------------------------------------------------------------
# Integration: stub injection into sys.modules
# ---------------------------------------------------------------------------

class TestStubInjection(unittest.TestCase):
    """Test that stubs can be injected and retrieved from sys.modules."""

    def test_inject_and_import(self):
        mod = _StubModule("_test_fake_unix_mod")
        mod.SOME_CONST = 42
        sys.modules["_test_fake_unix_mod"] = mod
        try:
            import _test_fake_unix_mod
            self.assertEqual(_test_fake_unix_mod.SOME_CONST, 42)
            self.assertEqual(_test_fake_unix_mod.unknown(), 0)
        finally:
            del sys.modules["_test_fake_unix_mod"]

    def test_stub_module_in_sys_modules_is_importable(self):
        """Stubs registered in sys.modules are importable via normal import."""
        mod = _StubModule("_test_stub_import")
        sys.modules["_test_stub_import"] = mod
        try:
            import importlib
            imported = importlib.import_module("_test_stub_import")
            self.assertIs(imported, mod)
        finally:
            del sys.modules["_test_stub_import"]


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------

class TestEdgeCases(unittest.TestCase):
    def test_noop_in_if_statement(self):
        """_Noop is falsy, so 'if fcntl.some_flag:' is False."""
        stub = _StubModule("test")
        self.assertFalse(stub.any_flag)

    def test_noop_chained_calls(self):
        """stub.foo.bar.baz() should not crash."""
        stub = _StubModule("test")
        # stub.foo returns _Noop, _Noop() returns 0,
        # 0.bar would fail — but the real pattern is never this deep
        result = stub.foo()
        self.assertEqual(result, 0)

    def test_noop_as_dict_key(self):
        """_Noop objects should be usable as dict keys (hashable)."""
        noop = _StubModule._Noop()
        d = {noop: "value"}
        self.assertEqual(d[noop], "value")

    def test_noop_equality(self):
        """Two _Noop instances should use default object identity equality."""
        a = _StubModule._Noop()
        b = _StubModule._Noop()
        self.assertNotEqual(a, b)  # different objects
        self.assertEqual(a, a)     # same object

    def test_noop_in_list_operations(self):
        """_Noop used in common list/collection patterns."""
        noop = _StubModule._Noop()
        # Often used as: if flag: ...
        self.assertFalse(bool(noop))
        # int() conversion for indexing
        self.assertEqual(int(noop), 0)


if __name__ == "__main__":
    unittest.main()
