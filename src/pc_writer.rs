use crate::bs_write::ByteStreamWriteBuffer;
use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::packet::DataPacketHeader;
use crate::paged_writer::PagedWriter;
use crate::CartesianBounds;
use crate::ColorLimits;
use crate::Error;
use crate::IndexBounds;
use crate::IntensityLimits;
use crate::PointCloud;
use crate::RawValues;
use crate::Record;
use crate::RecordDataType;
use crate::RecordName;
use crate::RecordValue;
use crate::Result;
use crate::SphericalBounds;
use std::collections::VecDeque;
use std::io::{Read, Seek, Write};

/// Creates a new point cloud by consuming points and writing them into an E57 file.
pub struct PointCloudWriter<'a, T: Read + Write + Seek> {
    writer: &'a mut PagedWriter<T>,
    pointclouds: &'a mut Vec<PointCloud>,
    guid: String,
    section_offset: u64,
    section_header: CompressedVectorSectionHeader,
    prototype: Vec<Record>,
    point_count: u64,
    buffer: VecDeque<RawValues>,
    max_points_per_packet: usize,
    cartesian_bounds: Option<CartesianBounds>,
    spherical_bounds: Option<SphericalBounds>,
    index_bounds: Option<IndexBounds>,
    color_limits: Option<ColorLimits>,
    intensity_limits: Option<IntensityLimits>,
}

impl<'a, T: Read + Write + Seek> PointCloudWriter<'a, T> {
    pub(crate) fn new(
        writer: &'a mut PagedWriter<T>,
        pointclouds: &'a mut Vec<PointCloud>,
        guid: &str,
        prototype: Vec<Record>,
    ) -> Result<Self> {
        // Make sure the prototype is not invalid or incomplete
        Self::validate_prototype(&prototype)?;

        let section_offset = writer.physical_position()?;

        let mut section_header = CompressedVectorSectionHeader::default();
        section_header.data_offset = section_offset + CompressedVectorSectionHeader::SIZE;
        section_header.section_length = CompressedVectorSectionHeader::SIZE;
        section_header.write(writer)?;

        // Each data packet can contain up to 2^16 bytes and we need some reserved
        // space for header and bytes that are not yet filled and need to be included later.
        let point_size: usize = prototype.iter().map(|p| p.data_type.bit_size()).sum();
        let max_points_per_packet = (64000 * 8) / point_size;

        // Prepare bounds
        let has_cartesian = prototype.iter().any(|p| p.name == RecordName::CartesianX);
        let cartesian_bounds = if has_cartesian {
            Some(CartesianBounds::default())
        } else {
            None
        };
        let has_spherical = prototype
            .iter()
            .any(|p| p.name == RecordName::SphericalAzimuth);
        let spherical_bounds = if has_spherical {
            Some(SphericalBounds::default())
        } else {
            None
        };
        let has_index = prototype.iter().any(|p| {
            p.name == RecordName::ReturnIndex
                || p.name == RecordName::ColumnIndex
                || p.name == RecordName::RowIndex
        });
        let index_bounds = if has_index {
            Some(IndexBounds::default())
        } else {
            None
        };

        // Prepare limits
        let has_color = prototype.iter().any(|p| p.name == RecordName::ColorRed);
        let color_limits = if has_color {
            let red_record = prototype
                .iter()
                .find(|p| p.name == RecordName::ColorRed)
                .internal_err("Unable to find red record")?;
            let green_record = prototype
                .iter()
                .find(|p| p.name == RecordName::ColorGreen)
                .internal_err("Unable to find green record")?;
            let blue_record = prototype
                .iter()
                .find(|p| p.name == RecordName::ColorBlue)
                .internal_err("Unable to find blue record")?;
            Some(ColorLimits::from_record_types(
                &red_record.data_type,
                &green_record.data_type,
                &blue_record.data_type,
            ))
        } else {
            None
        };
        let intensity = prototype.iter().find(|p| p.name == RecordName::Intensity);
        let intensity_limits = intensity.map(|i| IntensityLimits::from_record_type(&i.data_type));

        Ok(PointCloudWriter {
            writer,
            pointclouds,
            guid: guid.to_owned(),
            section_offset,
            section_header,
            prototype,
            point_count: 0,
            buffer: VecDeque::new(),
            max_points_per_packet,
            cartesian_bounds,
            spherical_bounds,
            index_bounds,
            color_limits,
            intensity_limits,
        })
    }

    fn validate_prototype(prototype: &[Record]) -> Result<()> {
        // Helper to look up if a records
        let contains = |n: RecordName| prototype.iter().any(|p| p.name == n);
        let get = |n: RecordName| prototype.iter().find(|p| p.name == n);

        // Cartesian coordinate check
        let mut cartesian = 0;
        if contains(RecordName::CartesianX) {
            cartesian += 1;
        }
        if contains(RecordName::CartesianY) {
            cartesian += 1;
        }
        if contains(RecordName::CartesianZ) {
            cartesian += 1;
        }
        if cartesian != 0 && cartesian != 3 {
            Error::invalid("You have to include all three Cartesian coordinates for X, Y and Z")?
        }
        if let Some(r) = get(RecordName::CartesianInvalidState) {
            if !contains(RecordName::CartesianX) {
                Error::invalid("CartesianInvalidState requires Cartesian coordinates")?
            }
            match r.data_type {
                RecordDataType::Integer { min: 0, max: 2 } => {}
                _ => {
                    Error::invalid("CartesianInvalidState needs to be an integer between 0 and 2")?
                }
            }
        }

        // Spherical coordinate check
        let mut spherical = 0;
        if contains(RecordName::SphericalAzimuth) {
            spherical += 1;
        }
        if contains(RecordName::SphericalElevation) {
            spherical += 1;
        }
        if contains(RecordName::SphericalRange) {
            spherical += 1;
        }
        if spherical != 0 && spherical != 3 {
            Error::invalid("You have to include all three spherical coordinates for azimuth, elevation and range")?
        }
        if let Some(r) = get(RecordName::SphericalInvalidState) {
            if !contains(RecordName::SphericalAzimuth) {
                Error::invalid("SphericalInvalidState requires spherical coordinates")?
            }
            match r.data_type {
                RecordDataType::Integer { min: 0, max: 2 } => {}
                _ => {
                    Error::invalid("SphericalInvalidState needs to be an integer between 0 and 2")?
                }
            }
        }
        if let Some(r) = get(RecordName::SphericalAzimuth) {
            if let RecordDataType::Integer { .. } = r.data_type {
                Error::invalid("SphericalAzimuth cannot have an integer type")?
            }
        }
        if let Some(r) = get(RecordName::SphericalElevation) {
            if let RecordDataType::Integer { .. } = r.data_type {
                Error::invalid("SphericalElevation cannot have an integer type")?
            }
        }

        // Cartesian or spherical?
        if !contains(RecordName::CartesianX) && !contains(RecordName::SphericalAzimuth) {
            Error::invalid("You have to include Cartesian or spherical coordinates")?
        }

        // Color check
        let mut color = 0;
        if contains(RecordName::ColorRed) {
            color += 1;
        }
        if contains(RecordName::ColorGreen) {
            color += 1;
        }
        if contains(RecordName::ColorBlue) {
            color += 1;
        }
        if color != 0 && color != 3 {
            Error::invalid("You have to include all three color values for red, green and blue")?
        }
        if let Some(r) = get(RecordName::IsColorInvalid) {
            if !contains(RecordName::ColorRed) {
                Error::invalid("IsColorInvalid requires colors")?
            }
            match r.data_type {
                RecordDataType::Integer { min: 0, max: 1 } => {}
                _ => Error::invalid("IsColorInvalid needs to be an integer between 0 and 1")?,
            }
        }

        // Return check
        let mut ret = 0;
        if let Some(r) = get(RecordName::ReturnCount) {
            ret += 1;
            match r.data_type {
                RecordDataType::Integer { .. } => {}
                _ => Error::invalid("ReturnCount must have an integer type")?,
            }
        }
        if let Some(r) = get(RecordName::ReturnIndex) {
            ret += 1;
            match r.data_type {
                RecordDataType::Integer { .. } => {}
                _ => Error::invalid("ReturnIndex must have an integer type")?,
            }
        }
        if ret != 0 && ret != 2 {
            Error::invalid("You have to include both, ReturnCount and ReturnIndex")?
        }

        // Row & column check
        if let Some(r) = get(RecordName::RowIndex) {
            match r.data_type {
                RecordDataType::Integer { .. } => {}
                _ => Error::invalid("RowIndex must have an integer type")?,
            }
        }
        if let Some(r) = get(RecordName::ColumnIndex) {
            match r.data_type {
                RecordDataType::Integer { .. } => {}
                _ => Error::invalid("ColumnIndex must have an integer type")?,
            }
        }

        // Intensity check
        if let Some(r) = get(RecordName::IsIntensityInvalid) {
            if !contains(RecordName::Intensity) {
                Error::invalid("IsIntensityInvalid requires Intensity")?
            }
            match r.data_type {
                RecordDataType::Integer { min: 0, max: 1 } => {}
                _ => Error::invalid("IsIntensityInvalid needs to be an integer between 0 and 1")?,
            }
        }

        // Time stamp check
        if let Some(r) = get(RecordName::IsTimeStampInvalid) {
            if !contains(RecordName::TimeStamp) {
                Error::invalid("IsTimeStampInvalid requires TimeStamp")?
            }
            match r.data_type {
                RecordDataType::Integer { min: 0, max: 1 } => {}
                _ => Error::invalid("IsTimeStampInvalid needs to be an integer between 0 and 1")?,
            }
        }

        Ok(())
    }

    fn write_buffer_to_disk(&mut self, last_write: bool) -> Result<()> {
        let packet_points = self.max_points_per_packet.min(self.buffer.len());
        if packet_points == 0 {
            return Ok(());
        }

        let prototype_len = self.prototype.len();
        let mut buffers = vec![ByteStreamWriteBuffer::new(); prototype_len];
        for _ in 0..packet_points {
            let p = self
                .buffer
                .pop_front()
                .internal_err("Failed to get next point for writing")?;
            for (i, prototype) in self.prototype.iter().enumerate() {
                let raw_value = p
                    .get(i)
                    .invalid_err("Prototype is bigger than number of provided values")?;
                prototype.data_type.write(raw_value, &mut buffers[i])?;
            }
        }

        // Check and prepare buffer sizes
        let mut sum_buffer_sizes = 0;
        let mut buffer_sizes = Vec::with_capacity(prototype_len);
        for buffer in &buffers {
            let len = if last_write {
                buffer.all_bytes()
            } else {
                buffer.full_bytes()
            };
            sum_buffer_sizes += len;
            buffer_sizes.push(len as u16);
        }

        // Calculate packet length for header
        let mut packet_length = DataPacketHeader::SIZE + prototype_len * 2 + sum_buffer_sizes;
        if packet_length % 4 != 0 {
            let missing = 4 - (packet_length % 4);
            packet_length += missing;
        }
        if packet_length > u16::MAX as usize {
            Error::internal("Invalid data packet length")?
        }

        // Add data packet length to section length for later
        self.section_header.section_length += packet_length as u64;

        // Write data packet header
        DataPacketHeader {
            comp_restart_flag: false,
            packet_length: packet_length as u64,
            bytestream_count: prototype_len as u16,
        }
        .write(&mut self.writer)?;

        // Write bytestream sizes as u16 values
        for size in buffer_sizes {
            let bytes = size.to_le_bytes();
            self.writer
                .write_all(&bytes)
                .write_err("Cannot write data packet buffer size")?;
        }

        // Write actual bytestream buffers with data
        for buffer in &mut buffers {
            let data = if last_write {
                buffer.get_all_bytes()
            } else {
                buffer.get_full_bytes()
            };
            self.writer
                .write_all(&data)
                .write_err("Cannot write bytestream buffer into data packet")?;
        }

        self.writer
            .align()
            .write_err("Failed to align writer on next 4-byte offset after writing data packet")?;

        Ok(())
    }

    /// Adds a new point to the point cloud.
    pub fn add_point(&mut self, values: RawValues) -> Result<()> {
        if values.len() != self.prototype.len() {
            Error::invalid("Number of values does not match prototype length")?
        }

        for (i, p) in self.prototype.iter().enumerate() {
            let value = &values[i];
            if !match p.data_type {
                RecordDataType::Single { .. } => matches!(value, RecordValue::Single(..)),
                RecordDataType::Double { .. } => matches!(value, RecordValue::Double(..)),
                RecordDataType::ScaledInteger { .. } => {
                    matches!(value, RecordValue::ScaledInteger(..))
                }
                RecordDataType::Integer { .. } => matches!(value, RecordValue::Integer(..)),
            } {
                Error::invalid(format!(
                    "Type mismatch at index {i}: value type does not match prototype"
                ))?
            }

            if p.name == RecordName::CartesianX
                || p.name == RecordName::CartesianY
                || p.name == RecordName::CartesianZ
            {
                let value = values[i].to_f64(&p.data_type)?;
                let bounds = self
                    .cartesian_bounds
                    .as_mut()
                    .internal_err("Cannot find cartesian bounds")?;
                if p.name == RecordName::CartesianX {
                    update_min(value, &mut bounds.x_min);
                    update_max(value, &mut bounds.x_max);
                }
                if p.name == RecordName::CartesianY {
                    update_min(value, &mut bounds.y_min);
                    update_max(value, &mut bounds.y_max);
                }
                if p.name == RecordName::CartesianZ {
                    update_min(value, &mut bounds.z_min);
                    update_max(value, &mut bounds.z_max);
                }
            }
            if p.name == RecordName::SphericalAzimuth
                || p.name == RecordName::SphericalElevation
                || p.name == RecordName::SphericalRange
            {
                let value = values[i].to_f64(&p.data_type)?;
                let bounds = self
                    .spherical_bounds
                    .as_mut()
                    .internal_err("Cannot find spherical bounds")?;
                if p.name == RecordName::SphericalAzimuth {
                    update_min(value, &mut bounds.azimuth_start);
                    update_max(value, &mut bounds.azimuth_end);
                }
                if p.name == RecordName::SphericalElevation {
                    update_min(value, &mut bounds.elevation_min);
                    update_max(value, &mut bounds.elevation_max);
                }
                if p.name == RecordName::SphericalRange {
                    update_min(value, &mut bounds.range_min);
                    update_max(value, &mut bounds.range_max);
                }
            }
            if p.name == RecordName::RowIndex
                || p.name == RecordName::ColumnIndex
                || p.name == RecordName::ReturnIndex
            {
                let value = values[i].to_i64(&p.data_type)?;
                let bounds = self
                    .index_bounds
                    .as_mut()
                    .internal_err("Cannot find index bounds")?;
                if p.name == RecordName::RowIndex {
                    update_min(value, &mut bounds.row_min);
                    update_max(value, &mut bounds.row_max);
                }
                if p.name == RecordName::ColumnIndex {
                    update_min(value, &mut bounds.column_min);
                    update_max(value, &mut bounds.column_max);
                }
                if p.name == RecordName::ReturnIndex {
                    update_min(value, &mut bounds.return_min);
                    update_max(value, &mut bounds.return_max);
                }
            }
        }

        self.buffer.push_back(values);
        self.point_count += 1;
        if self.buffer.len() >= self.max_points_per_packet {
            self.write_buffer_to_disk(false)?;
        }
        Ok(())
    }

    /// Called after all points have been added to finalize the creation of the new point cloud.
    pub fn finalize(&mut self) -> Result<()> {
        // Flush remaining points from buffer
        while !self.buffer.is_empty() {
            self.write_buffer_to_disk(true)?;
        }

        // We need to write the section header again with the final length
        // which was previously unknown and is now available.
        let end_offset = self
            .writer
            .physical_position()
            .write_err("Failed to get section end offset")?;
        self.writer
            .physical_seek(self.section_offset)
            .write_err("Failed to seek to section start for final update")?;
        self.section_header.write(&mut self.writer)?;
        self.writer
            .physical_seek(end_offset)
            .write_err("Failed to seek behind finalized section")?;

        // prepare point cloud metadata
        let pc = PointCloud {
            guid: self.guid.clone(),
            records: self.point_count,
            file_offset: self.section_offset,
            prototype: self.prototype.clone(),
            cartesian_bounds: self.cartesian_bounds.take(),
            spherical_bounds: self.spherical_bounds.take(),
            index_bounds: self.index_bounds.take(),
            color_limits: self.color_limits.take(),
            intensity_limits: self.intensity_limits.take(),
            ..Default::default()
        };

        // Add metadata for XML generation later, when the file is completed.
        self.pointclouds.push(pc);

        Ok(())
    }
}

fn update_min<T: PartialOrd>(value: T, min: &mut Option<T>) {
    if let Some(current) = min {
        if *current > value {
            *min = Some(value)
        }
    } else {
        *min = Some(value)
    }
}

fn update_max<T: PartialOrd>(value: T, min: &mut Option<T>) {
    if let Some(current) = min {
        if *current < value {
            *min = Some(value)
        }
    } else {
        *min = Some(value)
    }
}
