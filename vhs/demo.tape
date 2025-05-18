# Where should we write the GIF?
Output demo.gif

# Set up a 1200x600 terminal with 46px font.
Set FontSize 16
Set Width 1200
Set Height 1000

Type "pasejo store add --path /storage/path/to/store --name some-name"
Enter
Sleep 2s

Type "pasejo identity add --file /secret/identity"
Enter
Sleep 2s

Type "pasejo recipient add --file /public/recipient"
Enter
Sleep 2s

Type "pasejo secret generate some-secret"
Enter
Sleep 2s

Type "pasejo secret add very-secret"
Enter
Sleep 1s

Type "hunter2"
Enter
Sleep 1s

Type "hunter2"
Enter
Sleep 1s

Type "pasejo secret show very-secret"
Enter
Sleep 2s

Type "pasejo secret show some-secret"
Enter
Sleep 2s

Type "pasejo secret show some-secret --qrcode"
Enter
Sleep 5s
