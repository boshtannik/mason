#!/usr/bin/env python3
# Быстрая статическая проверка QML до сборки (без Qt-движка).
# Проверяет:
#   * баланс скобок {} () [] (строки и комментарии не считает);
#   * одинаковые id в одном файле (QML-ошибка "Duplicate id"). qmlscene:0.
# Полноценная валидация атрибутов/типов (qmllint) для SFOS 5.1 (Qt 5.6)
# недоступна; типовые ошибки атрибутов ловим динамикой: первому проходу QML —
# grep по device-логу "Cannot assign to non-existent property",
# "ReferenceError", "Unknown property" (см. smoke-скрипт деплоя).
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
QML_DIR = os.path.join(ROOT, "opencode-client", "qml")
IGNORE_DIRS = {".git", "target", "spike"}


def strip_mask(text):
    out = []
    i, n = 0, len(text)
    state = "code"
    while i < n:
        c = text[i]
        if state == "code":
            if text.startswith("//", i):
                state, i = "line", i + 2
                continue
            if text.startswith("/*", i):
                state, i = "block", i + 2
                continue
            if c == '"':
                state, i = "str", i + 1
                out.append(" ")
                continue
            out.append(c)
            i += 1
        elif state == "line":
            if c in "\n\r":
                state, i = "code", i + 1
                out.append(c)
            else:
                out.append(" ")
                i += 1
        elif state == "block":
            if text.startswith("*/", i):
                state, i = "code", i + 2
                out.append("  ")
            else:
                out.append(" ")
                i += 1
        else:  # строка
            if c == "\\" and i + 1 < n:
                i += 2
                out.append("  ")
                continue
            if c == '"':
                state, i = "code", i + 1
                out.append(" ")
            else:
                out.append(" ")
                i += 1
    return "".join(out)


def check_balance(code):
    stack = []
    pairs = {")": "(", "]": "[", "}": "{"}
    opens = set(pairs.values())
    for j, c in enumerate(code):
        if c in opens:
            stack.append(c)
        elif c in pairs:
            if not stack or stack[-1] != pairs[c]:
                return "unbalanced `{}` at offset {}; stack tail: {}".format(
                    c, j, "".join(stack[-4:]) or "empty")
            stack.pop()
    if stack:
        return "unclosed: {}".format("".join(stack[-4:]))
    return None


def main():
    errors = []
    files = []
    for base, dirs, names in os.walk(QML_DIR):
        dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]
        files.extend(os.path.join(base, n) for n in names if n.endswith(".qml"))
    files.sort()

    for f in files:
        rel = os.path.relpath(f, QML_DIR)
        try:
            raw = open(f, encoding="utf-8").read()
        except OSError as e:
            errors.append("{}: read: {}".format(rel, e))
            continue

        err = check_balance(strip_mask(raw))
        if err:
            errors.append("{}: syntax: {}".format(rel, err))

        ids = re.findall(r"\bid\s*:\s*([A-Za-z_]\w*)", raw)
        for x in sorted({x for x in ids if ids.count(x) > 1}):
            errors.append("{}: duplicate id: {}".format(rel, x))

    if errors:
        for e in errors:
            print("ERROR:", e)
        print("{} issue(s)".format(len(errors)))
        return 1
    print("OK: {} QML files, no syntax/dup-id issues".format(len(files)))
    return 0


if __name__ == "__main__":
    sys.exit(main())