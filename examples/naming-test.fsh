// Good naming conventions
Profile: GoodProfile
Parent: Patient
Id: good-profile
Title: "Good Profile"
Description: "This follows naming conventions"

Extension: GoodExtension
Id: good-extension
Title: "Good Extension"

// Bad naming conventions - should trigger warnings
Profile: bad_profile_name
Parent: Patient
Id: Bad_Profile_ID
Title: "Bad Profile"

Extension: my_bad_extension
Id: MyBadExtension
Title: "My Bad Extension"

ValueSet: bad_value_set
Id: BadValueSet

CodeSystem: My-Code-System
Id: my_code_system
