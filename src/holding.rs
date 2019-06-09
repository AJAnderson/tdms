let mut indices = Vec::new();
        for segment in &self.segments {
            // Do it the dumb way for now with unwraps
            let raw_data_index_root = segment.start_index as u64 + segment.raw_data_offset;
            let meta_data = segment
                .meta_data
                .as_ref()
                .ok_or("No meta data available in the segment".to_string())?;

            let channel_object = meta_data
                .objects
                .get(path)
                .ok_or("Channel does not exist".to_string())?;

            indices.push(channel_object.raw_data_index as u64 + raw_data_index_root);

            // Iterate over the objects contained in the metadata for the segment looking for the supplied path
            // segment.meta_data.as_mut().map(|meta_data| {
            //     let object = &meta_data.objects.get(path);
            //     match object {
            //         Some(object) => {
            //             // This is the part I know least about, this is apparently the index for where the raw data sits, I think it's within the raw data block?
            //             let mut root_index = object.raw_data_index;

            //             // Need to drill in and get the index if it's a string, otherwise need to know which object it was and the length of all other object data.
            //             object.properties.as_ref().map(|properties| {
            //                 for property in properties {
            //                     // The following must be able to be expressed more concisely, as far as I've seen there is no other viable indexing datatype
            //                     // for an NI_ArrayColumn, I expect this to hit an error on later test files
            //                     if property.prop_name.contains("NI_ArrayColumn") {
            //                         match property.property {
            //                             DataType::I32(arr_index) => {
            //                                 root_index = root_index + arr_index as u32;
            //                                 println!("Root_index: {}", root_index);
            //                             }
            //                             _ => unreachable!(),
            //                         }
            //                     }
            //                     // If there's no raw data type here it means it's because the object already existed in a past life? Carry over?
            //                     // TODO 1) If I need to know about past times I've seen this object if the data is carrying over do I use a hash map
            //                     // or do I carry over when I read?
            // match object.raw_data_type.expect("No raw_data_type for object") {
            //     DataTypeRaw::TdmsString => {
            //         let index = segment.start_index
            //             + segment.raw_data_offset as usize
            //             + root_index as usize;
            //         println!("String 1st Char Ptr Loc: {}", index);
            //         self.handle.seek(SeekFrom::Start(index as u64));
            //         self.read_datatype(DataTypeRaw::TdmsString);
            //     }
            //     _ => unreachable!(), // TODO: Only unreachable for my initial debug file
            // }
            //                 }
            //             });
            //             println!("No vals: {:?}", object.no_raw_vals);
            //         }
            //         None => println!("No channel data available for {}", path),
            //     }
            // });
        }