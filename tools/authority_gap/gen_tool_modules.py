"""Write, for each AgentDojo v1 suite, a Renyi project in which every tool is
a Python module of its own (so that each tool is a capability kind,
`python("tools.<tool>")`, the package name of the manifest): the declaration
files, each with the record types it returns declared inside it as `renyi
bind --python` writes them (a declaration file resolves no project import),
the manifest, and a smoke program that calls every tool once.

Usage: python -I gen_tool_modules.py <agentdojo tools dir> <out dir>

The suites' tool sets are AgentDojo's `task_suite.py` lists, copied here by
hand from those files; the signatures are read from the tools' AST.
"""
import ast
import sys
from pathlib import Path

SUITES = {
    "banking": {
        "modules": ["banking_client", "file_reader", "user_account"],
        "tools": [
            "get_iban", "send_money", "schedule_transaction", "update_scheduled_transaction",
            "get_balance", "get_most_recent_transactions", "get_scheduled_transactions",
            "read_file", "get_user_info", "update_password", "update_user_info",
        ],
    },
    "slack": {
        "modules": ["slack", "web"],
        "tools": [
            "get_channels", "add_user_to_channel", "read_channel_messages", "read_inbox",
            "send_direct_message", "send_channel_message", "invite_user_to_slack",
            "remove_user_from_slack", "get_users_in_channel", "post_webpage", "get_webpage",
        ],
    },
    "travel": {
        "modules": ["travel_booking_client", "email_client", "calendar_client"],
        "tools": [
            "get_user_information", "get_all_hotels_in_city", "get_hotels_prices",
            "get_hotels_address", "get_rating_reviews_for_hotels", "get_all_restaurants_in_city",
            "get_restaurants_address", "get_rating_reviews_for_restaurants",
            "get_cuisine_type_for_restaurants", "get_dietary_restrictions_for_all_restaurants",
            "get_contact_information_for_restaurants", "get_price_for_restaurants",
            "check_restaurant_opening_hours", "get_all_car_rental_companies_in_city",
            "get_car_types_available", "get_rating_reviews_for_car_rental",
            "get_car_rental_address", "get_car_fuel_options", "get_car_price_per_day",
            "reserve_hotel", "reserve_restaurant", "reserve_car_rental", "get_flight_information",
            "send_email", "get_day_calendar_events", "create_calendar_event", "search_calendar_events",
            "get_current_day", "cancel_calendar_event", "reschedule_calendar_event",
            "add_calendar_event_participants",
        ],
    },
    "workspace": {
        "modules": ["email_client", "calendar_client", "cloud_drive_client"],
        "tools": [
            "get_unread_emails", "send_email", "search_emails", "delete_email", "get_sent_emails",
            "get_received_emails", "get_draft_emails", "search_contacts_by_name",
            "search_contacts_by_email", "get_day_calendar_events", "create_calendar_event",
            "search_calendar_events", "get_current_day", "cancel_calendar_event",
            "reschedule_calendar_event", "add_calendar_event_participants",
            "search_files_by_filename", "create_file", "delete_file", "get_file_by_id",
            "list_files", "share_file", "append_to_file", "search_files",
        ],
    },
}

# the record types the tools return, with their fields as Renyi types
RECORDS = {
    "Transaction": [("id", "Integer"), ("sender", "Text"), ("recipient", "Text"), ("amount", "Float"),
                    ("subject", "Text"), ("date", "Text"), ("recurring", "Boolean")],
    "CalendarEvent": [("id", "Text"), ("title", "Text"), ("description", "Text"), ("start_time", "Text"),
                      ("end_time", "Text"), ("location", "maybe Text"), ("participants", "List of Text"),
                      ("all_day", "Boolean"), ("status", "Text")],
    "CloudDriveFile": [("id", "Text"), ("filename", "Text"), ("content", "Text"), ("owner", "Text"),
                       ("last_modified", "Text"), ("shared_with", "Map of Text to Text"), ("size", "Integer")],
    "EmailContact": [("email", "Text"), ("name", "Text")],
    "Email": [("id", "Text"), ("sender", "Text"), ("recipients", "List of Text"), ("cc", "List of Text"),
              ("bcc", "List of Text"), ("subject", "Text"), ("body", "Text"), ("status", "Text"),
              ("read", "Boolean"), ("timestamp", "Text"), ("attachments", "List of Text")],
    "Message": [("sender", "Text"), ("recipient", "Text"), ("body", "Text")],
}
RECORD_PURPOSE = {
    "Transaction": "A bank transaction: who sent what to whom, with its subject and date.",
    "CalendarEvent": "A calendar event with its times, location, participants and status.",
    "CloudDriveFile": "A file of the cloud drive with its content and who it is shared with.",
    "EmailContact": "A contact of the address book: an email address and a name.",
    "Email": "An email with its addresses, subject, body, status and attachments (as ids).",
    "Message": "A Slack message: its sender, its recipient (a user or a channel) and its body.",
}

SCALARS = {"str": "Text", "int": "Integer", "float": "Float", "bool": "Boolean", "EmailStr": "Text",
           "CloudDriveFileID": "Text", "EmailID": "Text", "CalendarEventID": "Text",
           "SharingPermission": "Text", "datetime.datetime": "Text", "datetime.date": "Text"}
DEPENDENCY_PARAMS = {"slack", "web"}


def renyi_type(annotation):
    """A Renyi type for a Python annotation text; None when it has no counterpart."""
    a = annotation.strip()
    if a in SCALARS:
        return SCALARS[a]
    if a in RECORDS:
        return a
    if a == "None":
        return None
    if a.endswith(" | None"):
        inner = renyi_type(a[: -len(" | None")])
        return f"maybe {inner}" if inner else None
    if a.startswith("list[") and a.endswith("]"):
        inner = renyi_type(a[5:-1])
        return f"List of {inner}" if inner else "List of Text"
    if a == "list":
        return "List of Text"
    if a.startswith("dict[") and a.endswith("]"):
        key, value = a[5:-1].split(",", 1)
        v = renyi_type(value)
        return f"Map of Text to {v or 'Text'}"
    if a == "dict":
        return "Map of Text to Text"
    return "Text"


def is_dependency(arg):
    if arg.arg in DEPENDENCY_PARAMS:
        return True
    return isinstance(arg.annotation, ast.Subscript) and ast.unparse(arg.annotation).startswith("Annotated[")


def tools_of(tools_dir, modules):
    found = {}
    for module in modules:
        tree = ast.parse((tools_dir / f"{module}.py").read_text(encoding="utf-8"))
        for node in tree.body:
            if not isinstance(node, ast.FunctionDef) or node.name.startswith("_"):
                continue
            params = []
            for arg in node.args.args:
                if is_dependency(arg):
                    continue
                # a single-letter name is refused by the checker; the bridge calls by position
                param = arg.arg if len(arg.arg) > 1 else {"n": "how_many"}.get(arg.arg, arg.arg + "_value")
                params.append((param, renyi_type(ast.unparse(arg.annotation)) if arg.annotation else "Text"))
            ret = renyi_type(ast.unparse(node.returns)) if node.returns else None
            doc = (ast.get_docstring(node) or "").strip().split("\n")[0].rstrip(".") or f"The tool {node.name}"
            doc = doc.split(". ")[0][:72].rstrip(".,; ")
            found[node.name] = (params, ret, doc)
    return found


def record(r):
    lines = [f"public type {r}", f"  purpose: {RECORD_PURPOSE[r]}"]
    for f, t in RECORDS[r]:
        lines.append(f"  has {f}: {t}")
    lines += ["  can ToJson", "  can FromJson", "end", ""]
    return lines


def declaration(name, params, ret, doc, records_used):
    lines = [f"module {name}", f"  purpose: The tool `{name}` of the environment: {doc}.", ""]
    lines.append("import std.python exposing PythonError")
    lines.append("")
    for r in sorted(records_used):
        lines += record(r)
    sig = ", ".join(f"{p}: {t}" for p, t in params)
    lines.append(f"public function {name}({sig})")
    if ret:
        lines.append(f"  returns {ret}")
    lines.append("  or fails with PythonError")
    lines.append(f'  needs python("tools.{name}")')
    lines.append(f"  purpose: {doc}.")
    lines.append("")
    return "\n".join(lines)


def records_in(types):
    used = set()
    for t in types:
        for r in RECORDS:
            if r in t.split():
                used.add(r)
    return used


def smoke(suite, tools):
    lines = [f"module smoke_{suite}", f"  purpose: Call every tool of the {suite} suite once, so that the declarations check.", ""]
    lines.append("import std.console")
    lines.append("import std.python exposing PythonError")
    for name in tools:
        lines.append(f"import {name}")
    lines.append("")
    names = list(tools)
    parts = [names[i : i + 12] for i in range(0, len(names), 12)]
    for number, part in enumerate(parts, 1):
        needs = ", ".join(f'python("tools.{name}")' for name in part)
        lines.append(f"function part_{number}() returns Integer or fails with PythonError needs {needs}")
        lines.append(f"  purpose: Call the tools of part {number} once and count the answers.")
        lines.append("")
        lines.append("  let mutable answered be 0")
        for name in part:
            params, ret, _ = tools[name]
            args = []
            for p, t in params:
                args.append(f"{p}: {sample(t)}")
            call = f"{name}.{name}(" + ", ".join(args) + ")" if len(args) != 1 else f"{name}.{name}({sample(params[0][1])})"
            if ret:
                lines.append(f"  ignore {call} otherwise fail")
            else:
                lines.append(f"  {call} otherwise fail")
            lines.append("  change answered to answered + 1")
        lines.append("  return answered")
        lines.append("end")
        lines.append("")
    needs = ", ".join(f'python("tools.{name}")' for name in names)
    lines.append(f"public function main() or fails with PythonError needs console, {needs}")
    lines.append("  purpose: Call every tool once and print how many answered.")
    lines.append("")
    total = " + ".join(f"(part_{number}() otherwise fail)" for number in range(1, len(parts) + 1))
    lines.append(f"  let answered be {total}")
    lines.append('  console.print("{answered} tools answered")')
    lines.append("end")
    return "\n".join(lines) + "\n"


def sample(t):
    if t == "Text":
        return '"x"'
    if t == "Integer":
        return "1"
    if t == "Float":
        return "1.0"
    if t == "Boolean":
        return "true"
    if t.startswith("maybe "):
        return "nothing"
    if t.startswith("List of "):
        return "[]"
    if t.startswith("Map of "):
        return "{}"
    return "nothing"


def main():
    tools_dir, out = Path(sys.argv[1]), Path(sys.argv[2])
    for suite, spec in SUITES.items():
        found = tools_of(tools_dir, spec["modules"])
        missing = [t for t in spec["tools"] if t not in found]
        if missing:
            sys.exit(f"{suite}: tools not found: {missing}")
        tools = {t: found[t] for t in spec["tools"]}
        d = out / suite
        d.mkdir(parents=True, exist_ok=True)
        records_used = set()
        for name, (params, ret, doc) in tools.items():
            used = records_in([t for _, t in params] + ([ret] if ret else []))
            records_used |= used
            (d / f"{name}.ry").write_text(declaration(name, params, ret, doc, used), encoding="utf-8", newline="\n")
        manifest = {
            "name": f"agentdojo_{suite}",
            "version": "0.1.0",
            "purpose": f"The tools of AgentDojo's {suite} suite as Python modules, one per tool.",
            "dependencies": {},
            "python": {"modules": {name: {"package": f"tools.{name}"} for name in tools}},
        }
        import json
        (d / "renyi.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
        (d / f"smoke_{suite}.ry").write_text(smoke(suite, tools), encoding="utf-8", newline="\n")
        print(f"{suite}: {len(tools)} tools, records {sorted(records_used)}")


main()
