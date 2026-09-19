# addrparts

I keep ending up with lists of US addresses as single free-form strings (CSV
exports, pasted from forms, scraped off web pages) and needing to know
whether they're actually well-formed before I do anything else with them.
`addrparts` takes one line, splits it into street / city / state / ZIP, and
tells you what's wrong with it if anything is.

It does not look anything up. There's no database of real streets or ZIP
codes behind this - it checks structure (comma layout, state code against
the USPS list, ZIP digit count) and nothing more. If you need to confirm an
address is deliverable, this isn't that tool. If you need to catch obviously
broken data before it hits a database, it is.

## Usage

```
addrparts "1600 Amphitheatre Pkwy, Mountain View, CA 94043"
```

```
street: 1600 Amphitheatre Pkwy
city:   Mountain View
state:  CA
zip:    94043
valid:  true
```

Or pipe it in:

```
echo "500 Elm St, Apt 4B, Austin, TX 73301" | addrparts
```

### Batch input

Piped input can hold more than one address, one per line. Blank lines are
skipped:

```
printf "123 Main St, Springfield, IL 62704\n1 First Ave, Nowhere, ZZ 00000\n" | addrparts
```

Human output prints each address's block separated by a blank line. With
`--json`, two or more addresses produce a JSON array instead of a single
object; a single address (piped or given as an argument) still produces one
object, unchanged from before. The exit code reflects whether *all* addresses
in the batch were valid.

### JSON output

```
addrparts --json "1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351"
```

```json
{"input":"1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351","valid":true,"address":{"street":"1600 Amphitheatre Pkwy","city":"Mountain View","state":"CA","zip5":"94043","zip4":"1351","po_box":null},"errors":[]}
```

On a bad address the JSON still comes back well-formed, with `valid: false`
and an `errors` array explaining why:

```
addrparts --json "1 First Ave, Nowhere, ZZ 00000"
```

```json
{"input":"1 First Ave, Nowhere, ZZ 00000","valid":false,"address":{"street":"1 First Ave","city":"Nowhere","state":"ZZ","zip5":"00000","zip4":null,"po_box":null},"errors":["'ZZ' is not a recognized state or territory code"]}
```

### PO boxes

When the street line is a PO box (`PO Box 123`, `P.O. Box 123`, `P O Box
123`, `Post Office Box 123`), the box number is pulled out into its own
`po_box` field alongside the usual `street` text - `street` still holds the
full original line. `--strict` doesn't require a USPS suffix abbreviation
on a PO box line, since there isn't one to check. A PO box line with no
number after it (`PO Box, Austin, TX 73301`) is invalid.

```
addrparts --json "PO Box 456, Austin, TX 73301"
```

```json
{"input":"PO Box 456, Austin, TX 73301","valid":true,"address":{"street":"PO Box 456","city":"Austin","state":"TX","zip5":"73301","zip4":null,"po_box":"456"},"errors":[]}
```

### Strict mode

`--strict` additionally requires the street to end in a standard USPS
suffix abbreviation (`St`, `Ave`, `Blvd`, `Pkwy`, ...) rather than a
spelled-out word. This catches "123 Main Street" as invalid, since USPS
Publication 28 wants "123 Main St".

```
addrparts --strict "123 Main Street, Springfield, IL 62704"
```

```
error:  'Street' is not a standard USPS street suffix abbreviation
valid:  false
```

A trailing period on the abbreviation (`St.`) is accepted. Extra folded
segments (apartment, suite) are not checked - only the first comma segment,
which is assumed to be the actual street line.

`--strict` also requires a directional (`North`, `Southwest`, ...) to be
abbreviated (`N`, `SW`, ...) if one appears in the street line at all -
prefix or suffix. Most streets don't have a directional, so this only fires
when one is actually present:

```
addrparts --strict "123 North Main St, Springfield, IL 62704"
```

```
error:  'North' should be abbreviated as 'N' in --strict mode
valid:  false
```

### Canonical single-line output

`--format` rejoins the parsed fields into a single line instead of printing
the usual report - useful for normalizing whitespace and casing (only the
state code comes back upper-cased; street and city are passed through as
typed) without hand-editing the original string:

```
addrparts --format "123 main st, springfield, il 62704"
```

```
123 main st, springfield, IL 62704
```

It reassembles whatever fields were split out even if the address failed
other validation (an unrecognized state code, say), since the fields are
still there to rejoin. An address that couldn't be split into fields at all
has nothing to reassemble - it's skipped with a message on stderr instead.
With batch input, `--format` prints one line per address. It takes
precedence over `--json` if both are given.

### Exit codes

- `0` - parsed and valid
- `1` - parsed but invalid (see `errors`)
- `2` - usage error (no input given)

## Expected input shape

```
STREET, CITY, STATE ZIP
STREET, CITY, STATE ZIP-ZIP4
```

Extra comma-separated segments before the last two (e.g. an apartment or
suite on its own segment) get folded back into the street field, so
`500 Elm St, Apt 4B, Austin, TX 73301` still parses correctly.

### Addresses without commas

If the input doesn't split into at least three comma segments, `addrparts`
falls back to a whitespace-only parse: the last two tokens are taken as
state and ZIP, and the street/city boundary is found by looking for the
rightmost token that matches a standard USPS suffix abbreviation (the same
list `--strict` checks against). That means

```
addrparts "123 Main St Springfield IL 62704"
```

parses the same as the comma-separated form. It only works when the street
ends in a recognizable abbreviation, though - `123 Main Street Springfield
IL 62704` has no structural marker for where the street name ends, so it
still fails to parse.

### Country suffix

A trailing country designator after the last comma (`USA`, `U.S.A.`, `US`,
`United States`, `United States of America` - matched case-insensitively)
is stripped before parsing, so it doesn't get mistaken for part of the
`STATE ZIP` tail:

```
addrparts "1600 Amphitheatre Pkwy, Mountain View, CA 94043, USA"
```

```
street: 1600 Amphitheatre Pkwy
city:   Mountain View
state:  CA
zip:    94043
valid:  true
```

The `input` field in JSON output still reflects what was actually passed
in, country suffix included.

## Building

```
cargo build --release
```

No third-party crates - standard library only.

## Limitations (known, not accidental)

- Assumes US addresses and the two-letter USPS state/territory code list.
- Assumes the street and city are unambiguous once the trailing
  `STATE ZIP` segment is peeled off. Addresses that omit commas entirely
  only parse if the street ends in a recognizable USPS suffix
  abbreviation - see "Addresses without commas" above.
- Doesn't validate that a ZIP code actually belongs to the given state or
  city.
