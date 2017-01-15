# -*- mode: ruby -*-
# vi: set ft=ruby :

# All Vagrant configuration is done below. The "2" in Vagrant.configure
# configures the configuration version (we support older styles for
# backwards compatibility). Please don't change it unless you know what
# you're doing.
Vagrant.configure("2") do |config|
  config.vm.box = "debian/wheezy64"

  config.vm.provider "virtualbox" do |vb|
    vb.memory = "2048"
    vb.cpus = 2
  end


  config.vm.define "old-debian" do |debian|
    debian.vm.box = "debian/wheezy64"

    config.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get install -y curl build-essential
      sudo -iu vagrant bash -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end

  config.vm.define "ubuntu-1610-32bit" do |ubuntu32|
    ubuntu32.vm.box = "box-cutter/ubuntu1610-i386"
    config.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get install -y curl build-essential
      sudo -iu vagrant bash -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end

  config.vm.define "ubuntu-1610-64bit" do |ubuntu64|
    ubuntu64.vm.box = "box-cutter/ubuntu1610"
    config.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get install -y curl build-essential
      sudo -iu vagrant bash -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end

  # This base box is buggy; you need to run `vagrant up freebsd-10-64` *twice*. The first time
  # it does with some networking configuration errors.
  # Bringing it up will hit issues near the end, but `vagrant ssh freebsd-10-64` does get you in.
  # Unfortunately that means it can't be auto provisioned. Run the following once ssh'd in:
  # sudo pkg install -y curl bash
  # curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
  # bash
  config.vm.define "freebsd-10-64" do |freebsd|
    freebsd.ssh.shell = '/bin/csh'
    freebsd.vm.box = "freebsd/FreeBSD-10.3-RELEASE"
  end
end
