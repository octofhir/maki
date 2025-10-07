// Test Multiline Strings - Task #013
Profile: MultilineTestProfile
Parent: Patient
Id: multiline-test-profile
Title: "Multiline Test Profile"
Description: """
This is a long
multiline description
that spans multiple lines.
"""

* name 1..* MS "Patient name" """
The patient's full legal name.
This can include multiple given names,
family names, prefixes, and suffixes.
"""

Extension: MultilineExtension
Id: multiline-extension
Title: "Multiline Extension"
Description: """
Line 1
Line 2
Line 3
"""

Instance: ExampleInstance
InstanceOf: Patient
Usage: #example
Description: """
This instance demonstrates
multiline string support
in FSH.
"""
