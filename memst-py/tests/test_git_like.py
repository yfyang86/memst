import pytest

from memst import Author, Commit, ObjectId, Tag, Tree


class TestTree:
    def test_tree_add_and_get_entry(self):
        tree = Tree()
        assert tree.entries == []

        oid = ObjectId(b"file-content")
        tree.add_entry(0o100644, oid.hex, "file.txt")

        assert len(tree.entries) == 1
        entry = tree.get_entry("file.txt")
        assert entry is not None
        assert entry.name == "file.txt"
        assert entry.oid == oid.hex
        assert entry.mode == 0o100644
        assert entry.mode_string == "100644"

        assert tree.get_entry("missing") is None
        assert "Tree(entries=1)" in repr(tree)

    def test_tree_add_entry_rejects_invalid_oid(self):
        tree = Tree()
        with pytest.raises(ValueError):
            tree.add_entry(0o100644, "not-hex", "file.txt")


class TestCommit:
    def test_commit_new_parent_and_merge(self):
        tree_oid = ObjectId(b"tree").hex
        author = Author("Alice", "alice@example.com")

        commit = Commit(tree_oid, author, "initial commit")
        assert commit.tree_oid == tree_oid
        assert commit.author.name == "Alice"
        assert commit.author.email == "alice@example.com"
        assert commit.message == "initial commit"
        assert commit.parent_oids == []
        assert commit.is_merge() is False
        assert "Commit(" in repr(commit)

        parent1 = ObjectId(b"p1").hex
        parent2 = ObjectId(b"p2").hex
        commit.add_parent(parent1)
        assert commit.parent_oids == [parent1]
        assert commit.is_merge() is False

        commit.add_parent(parent2)
        assert commit.parent_oids == [parent1, parent2]
        assert commit.is_merge() is True

    def test_commit_new_rejects_invalid_tree_oid(self):
        author = Author("Alice", "alice@example.com")
        with pytest.raises(ValueError):
            Commit("bad-oid", author, "msg")


class TestTag:
    def test_annotated_tag(self):
        target = ObjectId(b"target").hex
        tagger = Author("Bob", "bob@example.com")

        tag = Tag(target, "v0.1.0", tagger, "release")
        assert tag.target_oid == target
        assert tag.name == "v0.1.0"
        assert tag.tagger.name == "Bob"
        assert tag.tagger.email == "bob@example.com"
        assert tag.message == "release"
        assert tag.is_lightweight is False
        assert "Tag('v0.1.0'" in repr(tag)

    def test_lightweight_tag(self):
        target = ObjectId(b"target").hex

        tag = Tag.lightweight(target, "v0.1.0")
        assert tag.target_oid == target
        assert tag.name == "v0.1.0"
        assert tag.is_lightweight is True
        assert tag.message == ""
        assert tag.tagger.name == ""
        assert tag.tagger.email == ""

    def test_tag_rejects_invalid_target_oid(self):
        tagger = Author("Bob", "bob@example.com")
        with pytest.raises(ValueError):
            Tag("bad-oid", "v0.1.0", tagger, "msg")

        with pytest.raises(ValueError):
            Tag.lightweight("bad-oid", "v0.1.0")
