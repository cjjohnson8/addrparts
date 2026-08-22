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
{"input":"1600 Amphitheatre Pkwy, Mountain View, CA 94043-1351","valid":true,"address":{"street":"1600 Amphitheatre Pkwy","city":"Mountain View","state":"CA","zip5":"94043","zip4":"1351"},"errors":[]}
```

On a bad address the JSON still comes back well-formed, with `valid: false`
and an `errors` array explaining why:

```
addrparts --json "1 First Ave, Nowhere, ZZ 00000"
```

```json
{"input":"1 First Ave, Nowhere, ZZ 00000","valid":false,"address":{"street":"1 First Ave","city":"Nowhere","state":"ZZ","zip5":"00000","zip4":null},"errors":["'ZZ' is not a recognized state or territory code"]}
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

## Building

```
cargo build --release
```

No third-party crates - standard library only.

## Limitations (known, not accidental)

- Assumes US addresses and the two-letter USPS state/territory code list.
- Assumes the street and city are unambiguous once the trailing
  `STATE ZIP` segment is peeled off. Addresses that omit commas entirely
  won't parse.
- Doesn't validate that a ZIP code actually belongs to the given state or
  city.
