
    #[test]
    fn test_decoder_cursor_read_operations() {
        let mut cursor = Cursor::new(&[0x01, 0xFF, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00]);

        assert!(cursor.has_remaining());
        assert_eq!(cursor.read_u8().unwrap(), 0x01);
        assert_eq!(cursor.read_i8().unwrap(), -1);
        assert_eq!(cursor.read_u16().unwrap(), 0x0002);
        assert_eq!(cursor.read_i32().unwrap(), -1);

        // At position 8. Buffer is length 10. `has_remaining` should be true.
        assert!(cursor.has_remaining());
        let _ = cursor.read_u16().unwrap();
        // Now position 10.
        assert!(!cursor.has_remaining());

        // Out of bounds read.
        assert!(matches!(cursor.read_u8(), Err(crate::Error::Decode(DecodeError::UnexpectedEof { pc: 10 }))));

        // align4 test
        let mut cursor2 = Cursor::new(&[0x00, 0x01]);
        assert_eq!(cursor2.read_u8().unwrap(), 0x00);
        cursor2.align4();
        // Since pos is 1, rem is 1. align4 adds 3, making pos 4.
        assert_eq!(cursor2.pos, 4);

        // cp read
        let mut cursor3 = Cursor::new(&[0x00, 0x42]);
        assert_eq!(cursor3.read_cp().unwrap().0, 0x42);

        // i16 test
        let mut cursor4 = Cursor::new(&[0xFF, 0xFF]);
        assert_eq!(cursor4.read_i16().unwrap(), -1);
    }
