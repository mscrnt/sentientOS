@echo -off
echo "SentientOS Boot Script"
echo "====================="
echo ""
echo "Listing available files:"
ls
echo ""
echo "Checking for bootloader:"
if exist EFI\BOOT\BOOTX64.EFI then
  echo "Found BOOTX64.EFI, launching..."
  EFI\BOOT\BOOTX64.EFI
else
  echo "ERROR: BOOTX64.EFI not found!"
  echo "Contents of EFI directory:"
  ls EFI
endif