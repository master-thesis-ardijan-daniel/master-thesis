# A Real-Time 3D Earth, Written in Rust with wGPU, Deployed via WebAssembly and Nix: A Unified, Efficient and Reproducible System for Raster Data Visualization and Analysis



![Application screenshot](/documents/readme_background.png)

# [globe.devmode.app](https://globe.devmode.app)


![image](https://github.com/user-attachments/assets/792d936c-907e-40b4-93bc-bab2e0178e03)



# Instructions

### NixOS
On systems with Nix, the complete application can be built using the provided Nix flake. Clone the GitHub repository using:

```
git clone https://github.com/master-thesis-ardijan-daniel/master-thesis
```

Open a terminal inside the folder of the repository and run:

```
nix run .#
```

This will run the application on \verb|localhost:8000|. If the application cannot find the required datasets, such as NASA's \textit{Blue marble} satellite data, the application will panic when it attempts to build the database. You can register and download the required data from NASA's website for free, then set the required environment variable called "EARTH\_MAP\_DATASET", which will allow you to build the database and run the application.


### Other systems
For any other system, pull the container image from the GitHub container registry. The following examples use Docker:


```
docker pull ghcr.io/master-thesis-ardijan-daniel/master_thesis:latest
```

This image can then be used in a Docker Compose file or ran directly:

```
docker run --rm \
  -p 8000:8000 \
  -e EARTH_MAP_DATASET="<Path in container>" \
  -v <path on host>:<path in container> \
  ghcr.io/master-thesis-ardijan-daniel/master_thesis:latest
```

Note that you need to attach the file or folder where the dataset is contained and set the container folder path in the environment variable. Once the application has built and the database is running, you can access the website on \verb|localhost:8000|.
