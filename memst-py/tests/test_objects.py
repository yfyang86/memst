from memst import Blob, ObjectId, TreeEntry


class TestObjectId:
    def test_nil_and_roundtrip(self):
        nil = ObjectId.nil()
        assert nil.is_nil() is True
        assert isinstance(nil.hex, str)
        assert len(nil.hex) == 64
        assert set(nil.hex) == {"0"}

        oid = ObjectId(b"hello")
        assert oid.is_nil() is False
        assert len(oid.hex) == 64

        oid2 = ObjectId.from_hex(oid.hex)
        assert oid2.hex == oid.hex

    def test_abbreviate_and_repr(self):
        oid = ObjectId(b"content")
        abbr = oid.abbreviate()
        assert isinstance(abbr, str)
        assert len(abbr) == 7
        assert oid.hex.startswith(abbr)

        r = repr(oid)
        assert r.startswith("ObjectId('")


class TestBlob:
    def test_blob_content_roundtrip(self):
        b = Blob(b"abc")
        assert b.content == [97, 98, 99]


class TestTreeEntry:
    def test_tree_entry_mode_string(self):
        oid = ObjectId(b"file")
        entry = TreeEntry(0o100644, oid.hex, "file.txt")
        assert entry.mode == 0o100644
        assert entry.mode_string == "100644"
        assert entry.oid == oid.hex
        assert entry.name == "file.txt"
