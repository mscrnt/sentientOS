@echo -off
echo "SentientOS Boot Script"
echo "====================="
echo ""
FS0:
echo "Switched to FS0:"
echo ""
echo "Listing available files:"
ls
echo ""
echo "Launching bootloader..."
EFI\BOOT\BOOTX64.EFI