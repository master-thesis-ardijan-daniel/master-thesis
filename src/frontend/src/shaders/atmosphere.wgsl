


struct Light{

    // How much of rgb light is coming in
    wavelength_intensity: vec3<f32>,
    source_position: vec3<f32>,
    
}


fn atmospheric_scattering(camera_position:Camera)-> LightOut{
    // Difficult to make one which covers all cases, its too slow due to the
    // shear number of computations per pixel
    // in theory each pixel would need to calculate the scattering from each light source
    // Meaning we would have to loop through all light sources and if the light source contributes
    // add its contribution through the scattering function
    // This would have to be done through multiple times along each pixel ray
    //
    //
    // in order to simlify we can use 2 additional shapes outside atmosphere and inside atmosphere
    // which are then combined and give us total atmospheric thickness in the visible parts of the atmosphere
    //
    // given that we have total atmospheric thickness.
    //
    // We still need to calculate the scattering along each point along that thickness
    // from each light source
    //
    //
    // 
}





fn sky_shader(){
    
}


fn lp_shader(){
    
}
