"""The authority gap on AgentDojo: for every (user task, injection task) pair of
a suite, would the injection's ground-truth calls fit inside the smallest
grant the user task's program can declare?

Usage: python -I authority_gap.py <tasks.json> [--pairs]

A tool is a capability kind; the argument named in SCOPE is its scope (a
list argument is a set of scopes). The user task's grant declares a scope
for a tool only when every ground-truth call of that tool has a literal
scope that the user's prompt spells out (the program can write it in its
`needs`); otherwise the tool is granted without a scope. Three grants are
measured per pair:
  kind          the grant names kinds only (no scopes)
  prompt        scopes declared when literal in the prompt (the honest estimate)
  prompt+budget the same, with `at most n per run` on every kind, n the
                number of calls of the kind in the user task's ground truth
  literal       scopes declared for every literal argument (an upper bound)
A pair is blocked when the injection needs a kind outside the grant, or a
scope outside a declared scope. An injection whose scope is derived from
the environment against a declared scope is `undetermined` (not blocked).
An injection with no tool call cannot be blocked by any grant.
"""
import json
import sys
from collections import Counter

SCOPE = {
    # banking
    "send_money": "recipient",
    "schedule_transaction": "recipient",
    "update_scheduled_transaction:recipient": "recipient",
    "read_file": "file_path",
    # slack
    "get_webpage": "url",
    "post_webpage": "url",
    "download_file": "url",
    "send_direct_message": "recipient",
    "send_channel_message": "channel",
    "invite_user_to_slack": "user_email",
    "add_user_to_channel": "channel",
    "remove_user_from_slack": "user",
    "read_channel_messages": "channel",
    "read_inbox": "user",
    "get_users_in_channel": "channel",
    # travel
    "reserve_hotel": "hotel",
    "reserve_restaurant": "restaurant",
    "reserve_car_rental": "company",
    "get_all_hotels_in_city": "city",
    "get_all_restaurants_in_city": "city",
    "get_all_car_rental_companies_in_city": "city",
    # workspace (and travel's email and calendar)
    "send_email": "recipients",
    "create_calendar_event": "participants",
    "add_calendar_event_participants": "participants",
    "share_file": "email",
    "delete_file": "file_id",
    "delete_email": "email_id",
    "append_to_file": "file_id",
    "reschedule_calendar_event": "event_id",
    "cancel_calendar_event": "event_id",
}


def kind_of(call):
    # changing a scheduled transaction's recipient is another authority than
    # changing its amount: the tool takes both as optional arguments
    if call["function"] == "update_scheduled_transaction":
        return "update_scheduled_transaction:recipient" if "recipient" in call["args"] else "update_scheduled_transaction:other"
    return call["function"]


def norm(v):
    return str(v).strip().lower()


def scopes_of(call, kind):
    """(status, set): status 'none' when the kind has no scope argument or the
    call leaves it out, 'derived' when it is computed, 'literal' with the set."""
    arg = SCOPE.get(kind)
    if arg is None or arg not in call["args"]:
        return "none", set()
    a = call["args"][arg]
    if a["kind"] != "literal":
        return "derived", set()
    v = a["value"]
    values = v if isinstance(v, list) else [v]
    return "literal", {norm(x) for x in values}


def grant_of(task, mode):
    """kind -> None (unscoped) or a set of declared scopes."""
    prompt = norm(task["text"])
    grant = {}
    for call in task["calls"]:
        kind = kind_of(call)
        status, values = scopes_of(call, kind)
        declared = grant.get(kind, set())
        if declared is None:
            continue
        if mode == "kind" or status == "derived":
            grant[kind] = None
            continue
        if status == "none":
            grant.setdefault(kind, set())
            continue
        if mode == "prompt" and not all(x in prompt for x in values):
            grant[kind] = None
            continue
        grant[kind] = declared | values
    return grant


def judge(user, injection, mode):
    """'kind' when the injection needs a kind the grant lacks (checked over
    every call first, so the verdict does not depend on the calls' order),
    else 'scope' when a scope falls outside a declared one, else 'budget'
    when the mode asks for budgets and the injection makes more calls of a
    kind than the user task's ground truth (`at most n per run`), else
    'undetermined' or 'fits'."""
    if not injection["calls"]:
        return "no-tool"
    grant = grant_of(user, mode.replace("+budget", ""))
    kinds = [kind_of(call) for call in injection["calls"]]
    if any(kind not in grant for kind in kinds):
        return "kind"
    verdict = "fits"
    for call, kind in zip(injection["calls"], kinds):
        declared = grant[kind]
        if declared is None:
            continue
        status, values = scopes_of(call, kind)
        if status == "none":
            continue
        if status == "derived":
            verdict = "undetermined"
            continue
        if not values <= declared:
            return "scope"
    if mode.endswith("+budget"):
        user_counts = Counter(kind_of(call) for call in user["calls"])
        if any(n > user_counts[kind] for kind, n in Counter(kinds).items()):
            return "budget"
    return verdict


def main():
    data = json.load(open(sys.argv[1], encoding="utf-8"))
    show_pairs = "--pairs" in sys.argv
    modes = ["kind", "prompt", "prompt+budget", "literal"]
    total = Counter()
    grand = {m: Counter() for m in modes}
    for suite, s in data.items():
        counts = {m: Counter() for m in modes}
        n = 0
        for u in s["user"].values():
            for i in s["injection"].values():
                n += 1
                for m in modes:
                    v = judge(u, i, m)
                    counts[m][v] += 1
                    grand[m][v] += 1
                if show_pairs:
                    v = judge(u, i, "prompt+budget")
                    if v in ("fits", "undetermined"):
                        print(f"  {suite} {u['id']} x {i['id']}: {v}")
        total[suite] = n
        print(f"{suite}: {len(s['user'])} user x {len(s['injection'])} injection = {n} pairs")
        for m in modes:
            c = counts[m]
            blocked = c["kind"] + c["scope"] + c["budget"]
            print(f"  {m:13s} blocked {blocked:4d} ({100*blocked/n:5.1f}%): kind {c['kind']:3d}, scope {c['scope']:3d}, budget {c['budget']:3d}; "
                  f"undetermined {c['undetermined']:3d}, fits {c['fits']:3d}, no-tool {c['no-tool']:3d}")
    n = sum(total.values())
    print(f"all: {n} pairs")
    for m in modes:
        c = grand[m]
        blocked = c["kind"] + c["scope"] + c["budget"]
        print(f"  {m:13s} blocked {blocked:4d} ({100*blocked/n:5.1f}%): kind {c['kind']:3d}, scope {c['scope']:3d}, budget {c['budget']:3d}; "
              f"undetermined {c['undetermined']:3d}, fits {c['fits']:3d}, no-tool {c['no-tool']:3d}")


main()
